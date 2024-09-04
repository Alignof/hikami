//! Utility for Hypervisor extension.

pub mod csrs;
pub mod instruction;

/// Exception type in H extension.
pub enum HvException {
    /// Environment call from VS-mode
    EcallFromVsMode = 10,
    /// Load guest-page fault
    LoadGuestPageFault = 21,
    /// Virtual instruction
    VirtualInstruction = 22,
    /// Store/AMO guest-page fault
    StoreAmoGuestPageFault = 23,
}

impl From<usize> for HvException {
    fn from(exception_num: usize) -> Self {
        match exception_num {
            10 => HvException::EcallFromVsMode,
            21 => HvException::LoadGuestPageFault,
            22 => HvException::VirtualInstruction,
            23 => HvException::StoreAmoGuestPageFault,
            _ => panic!("unsupported exception number: {exception_num}"),
        }
    }
}
