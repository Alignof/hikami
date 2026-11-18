//! Utility for Hypervisor extension.

pub mod csrs;
pub mod instruction;

/// Exception type in H extension.
pub enum HvException {
    /// Environment call from VS-mode
    EcallFromVsMode = 10,
    /// Instruction guest-page fault
    InstructionGuestPageFault = 20,
    /// Load guest-page fault
    LoadGuestPageFault = 21,
    /// Virtual instruction
    VirtualInstruction = 22,
    /// Store/AMO guest-page fault
    StoreAmoGuestPageFault = 23,
}

impl From<riscv::result::Error> for HvException {
    fn from(convert_err: riscv::result::Error) -> Self {
        if let riscv::result::Error::InvalidVariant(exception_num) = convert_err {
            match exception_num {
                10 => HvException::EcallFromVsMode,
                20 => HvException::InstructionGuestPageFault,
                21 => HvException::LoadGuestPageFault,
                22 => HvException::VirtualInstruction,
                23 => HvException::StoreAmoGuestPageFault,
                _ => panic!("unsupported exception number: {exception_num}"),
            }
        } else {
            panic!("Tried to retrieve exception value from other error code.")
        }
    }
}
