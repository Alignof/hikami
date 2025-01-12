//! Utility for HBA (= ATA) command.

/// Size of command header
pub const COMMAND_HEADER_SIZE: usize = 0x20;

/// HBA command header
///
/// Ref: [https://wiki.osdev.org/AHCI#AHCI_Registers_and_Memory_Structures](https://wiki.osdev.org/AHCI#AHCI_Registers_and_Memory_Structures): 4) Command List
#[repr(C)]
pub struct CommandHeader {
    /// DW0: PRDTL, PMP, etc...
    ///
    /// PRDTL[31:16] PMP[15:12] * C B R P W A CFL[4:0]
    /// *: Reserved
    dw0: u32,
    /// DW1: PRD Byte Count
    prdbc: u32,
    /// DW2: Command Table Base Address
    pub ctba: u32,
    /// DW3: Command Table Base Address upper 32 bits
    pub ctba_u: u32,
    /// DW4-7: Reserved
    _reserved: [u32; 4],
}

impl CommandHeader {
    pub fn prdtl(&self) -> u32 {
        self.dw0 >> 16 & 0xffff
    }
}
