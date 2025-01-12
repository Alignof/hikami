//! Utility for HBA (= ATA) command.

use crate::memmap::page_table::g_stage_trans_addr;
use crate::memmap::GuestPhysicalAddress;

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

/// HBA physical region descriptor table item
struct PhysicalRegionDescriptor {
    /// Data Base Address
    dba: u32,
    /// Data Base Address Upper 32-bits
    dbau: u32,
    /// Reserved
    _reserved: u32,
    /// Data Byte Count
    _dbc: u32,
}

impl PhysicalRegionDescriptor {
    pub fn translate_data_base_address(&mut self) {
        let db_gpa = GuestPhysicalAddress((self.dbau as usize) << 32 | self.dba as usize);
        let db_hpa = g_stage_trans_addr(db_gpa).expect("data base address translation failed");
        self.dbau = ((db_hpa.raw() >> 32) & 0xffff_ffff) as u32;
        self.dba = (db_hpa.raw() & 0xffff_ffff) as u32;
    }
}

/// HBA command table
///
/// Ref: [https://wiki.osdev.org/AHCI#AHCI_Registers_and_Memory_Structures](https://wiki.osdev.org/AHCI#AHCI_Registers_and_Memory_Structures): 4) Command List
#[repr(C)]
pub struct CommandTable {
    /// Command FIS
    _cfis: [u8; 0x40],
    /// ATAPI Command
    _acmd: [u8; 0x10],
    /// Reserved
    _reserved: [u8; 0x30],
    /// Physical Region Descriptor Table
    ///
    /// Actual size is 0 .. PRDTL (defined in `CommandHeader`)
    prdt: [PhysicalRegionDescriptor; 1],
}

impl CommandTable {
    pub fn translate_all_data_base_addresses(&mut self, prdtl: u32) {
        let prdt_ptr = self.prdt.as_mut_ptr() as *mut PhysicalRegionDescriptor;
        for index in 0..prdtl {
            unsafe {
                let prd_ptr = prdt_ptr.add(index as usize);
                (*prd_ptr).translate_data_base_address();
            }
        }
    }
}
