#![allow(dead_code)]

#[repr(u8)]
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Vector {
    DivideError = 0,
    Debug = 1,
    Nmi = 2,
    Breakpoint = 3,
    Overflow = 4,
    BoundRange = 5,
    InvalidOpcode = 6,
    DeviceNotAvail = 7,
    DoubleFault = 8,
    // 9 is reserved (coprocessor segment overrun, obsolete)
    InvalidTss = 10,
    SegmentNotPresent = 11,
    StackSegment = 12,
    GeneralProtection = 13,
    PageFault = 14,
    // 15 reserved
    X87FloatingPoint = 16,
    AlignmentCheck = 17,
    MachineCheck = 18,
    SimdFloat = 19,
    Virtualization = 20,
    ControlProtection = 21,
    // 22-31 reserved
}
