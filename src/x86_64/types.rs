#![allow(dead_code)]

#[repr(C, packed)]
pub(super) struct DescriptorTablePointer {
    pub(super) limit: u16,
    pub(super) base: u64,
}

#[repr(u8)]
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub(super) enum GateType {
    TaskGate = 0x5,
    InterruptGate = 0xE,
    TrapGate = 0xF,
}

#[repr(u8)]
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub(super) enum Dpl {
    Kernel = 0,
    User = 3,
}
