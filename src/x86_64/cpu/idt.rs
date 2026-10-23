// refer to AMD APM Vol 2. chapter 8 or Intel SDM vol 3 chapter 6
// Interrupts and exceptions are events that indicate that a condition exists somewhere in the
// system, the processor, or within the currently executing program or task that requires the
// attention of a processor.

// The IDT can contain only the following type of gate descriptors:
// - Interrupt gates.
// - Trap gates.
// - Task gates.

// note: The target code segment referenced by the interrupt gate must be a 64-bit code segment (CS.L = 1, CS.D = 0). If
// the target is not a 64-bit code segment, a general-protection exception (#GP) is generated with the IDT vector
// number reported as the error code.

//+--------+----------+---------------------------------------+-----------+------------+
// | Vector | Mnemonic | Description                           | Type      | Error Code |
// +--------+----------+---------------------------------------+-----------+------------+
// | 0      | #DE      | Divide Error                          | Fault     | No         |
// | 1      | #DB      | Debug Exception                       | Fault/Trap| No         |
// | 2      | NMI      | Non-Maskable Interrupt                | Interrupt | No         |
// | 3      | #BP      | Breakpoint                            | Trap      | No         |
// | 4      | #OF      | Overflow                              | Trap      | No         |
// | 5      | #BR      | BOUND Range Exceeded                  | Fault     | No         |
// | 6      | #UD      | Invalid Opcode                        | Fault     | No         |
// | 7      | #NM      | Device Not Available                  | Fault     | No         |
// | 8      | #DF      | Double Fault                          | Abort     | Yes (0)    |
// | 9      | -        | Coprocessor Segment Overrun (Reserved)| Fault     | No         |
// | 10     | #TS      | Invalid TSS                           | Fault     | Yes        |
// | 11     | #NP      | Segment Not Present                   | Fault     | Yes        |
// | 12     | #SS      | Stack Segment Fault                   | Fault     | Yes        |
// | 13     | #GP      | General Protection                    | Fault     | Yes        |
// | 14     | #PF      | Page Fault                            | Fault     | Yes        |
// | 15     | -        | Intel Reserved                        | -         | No         |
// | 16     | #MF      | x87 Floating-Point Error              | Fault     | No         |
// | 17     | #AC      | Alignment Check                       | Fault     | Yes (0)    |
// | 18     | #MC      | Machine Check                         | Abort     | No         |
// | 19     | #XM/#XF  | SIMD Floating-Point Exception         | Fault     | No         |
// | 20     | #VE      | Virtualization Exception              | Fault     | No         |
// | 21-31  | -        | Intel Reserved                        | -         | No         |
// | 32-255 | User     | External Interrupts (IRQ/MSI/MSI-X)   | Interrupt | Depends    |
// +--------+----------+---------------------------------------+-----------+------------+

use crate::x86_64::{
    cpu::gdt::KERNEL_CODE_SELECTOR,
    interrupts::exceptions::{do_general_protection_fault, do_page_fault},
};
use core::arch::asm;

use crate::{
    sync::once_lock::OnceLock,
    x86_64::{
        interrupts::{frame::InterruptStackFrame, vector::Vector},
        types::{DescriptorTablePointer, Dpl, GateType},
    },
};

#[allow(unused)]
pub(super) type IdtHandler = extern "x86-interrupt" fn(InterruptStackFrame);
pub(super) type IdtHandlerWithErrorCode = extern "x86-interrupt" fn(InterruptStackFrame, u64);

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub(super) struct IdtEntry([u32; 4]);

impl IdtEntry {
    #[inline]
    pub(super) const fn new(
        handler_addr: u64,
        dpl: Dpl,
        seg_selector: u16,
        gate_type: GateType,
    ) -> Self {
        // lower dword offset (0)
        // 15:0 -> offset 15..0 of our exception handler virtual address
        // 31:16 -> Segment Selector for destination code segment
        let low: u32 = ((seg_selector as u32) << 16) | ((handler_addr & 0xFFFF) as u32);

        // upper dword (4)
        // 2:0 -> IST (Interrupt stack table)
        // 7:3 -> 0 (reserved)
        // 11:8 -> Type (0101 for Task Gate, 1110 for Interrupt Gate, 1111 for Trap Gate)
        // 12 -> 0
        // 14:13 -> DPL
        // 15 -> Segment Present Flag
        // 16:31 -> offset 31..16 of our exception handler virtual address
        let mid: u32 = ((gate_type as u32) << 8)
            | ((dpl as u32) << 13)
            | (((handler_addr as u32) >> 16) & 0xFFFF) << 16
            | 0x8000;

        // upper dword offset (8)
        // 31:0 -> offset 63..32 of out exception handler virtual address
        let high: u32 = ((handler_addr >> 32) as u32) & 0xFFFFFFFF;

        Self {
            0: [low, mid, high, 0],
        }
    }
}

#[repr(C, align(16))]
pub(super) struct Idt([IdtEntry; 256]);

impl Idt {
    pub(super) const fn new() -> Self {
        Self([IdtEntry([0, 0, 0, 0]); 256])
    }

    #[inline]
    pub(super) fn register(&mut self, vector: Vector, entry: IdtEntry) {
        self.0[vector as usize] = entry;
    }
}

static IDT: OnceLock<Idt> = OnceLock::new();

pub fn load_idt() {
    let pf_entry = IdtEntry::new(
        do_page_fault as IdtHandlerWithErrorCode as u64,
        Dpl::Kernel,
        KERNEL_CODE_SELECTOR,
        GateType::InterruptGate,
    );

    let gf_entry = IdtEntry::new(
        do_general_protection_fault as IdtHandlerWithErrorCode as u64,
        Dpl::Kernel,
        KERNEL_CODE_SELECTOR,
        GateType::InterruptGate,
    );

    let mut idt = Idt::new();
    idt.register(Vector::PageFault, pf_entry);
    idt.register(Vector::GeneralProtection, gf_entry);

    unsafe {
        IDT.get_or_init(|| idt);

        let idtr = DescriptorTablePointer {
            limit: ((256 * 16) - 1),
            base: core::ptr::addr_of!(IDT) as u64,
        };

        asm!(
            "lidt [{}]",
            in(reg) &idtr as *const DescriptorTablePointer,
            options(readonly, nostack, preserves_flags)
        );
    }
}
