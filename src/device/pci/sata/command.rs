//! Utility for HBA (= ATA) command.

use crate::memmap::page_table::{constants::PAGE_SIZE, g_stage_trans_addr};
use crate::memmap::GuestPhysicalAddress;

use alloc::vec::Vec;

/// Size of command header
pub const COMMAND_HEADER_SIZE: usize = 0x20;

/// Transfer direction
#[derive(PartialEq)]
pub enum TransferDirection {
    /// Device to Host (Read).
    DeviceToHost,
    /// Host to Device (Write).
    HostToDevice,
}

/// Address of command table to be saved.
#[derive(Debug, Clone)]
pub enum CommandTableAddressData {
    /// Address before translation (Memory block size <= 0x1000).
    TranslatedAddress(GuestPhysicalAddress),
    /// Address before replacing to allocated memory region  (Memory block size > 0x1000).
    AllocatedAddress(GuestPhysicalAddress, Vec<u8>),
}

/// Addresses of `CommandTable` and its each CTBA.
///
/// It will be used at address restoring.
#[derive(Debug, Clone)]
pub struct CommandTableGpaStorage {
    /// Address of Command Table Structure
    pub cmd_table_gpa: GuestPhysicalAddress,
    /// List of CTBA (Command Table Base Address)
    pub ctba_list: Vec<CommandTableAddressData>,
}

impl CommandTableGpaStorage {
    /// Generate new `CommandTableGpaStorage`.
    pub const fn new() -> Self {
        CommandTableGpaStorage {
            cmd_table_gpa: GuestPhysicalAddress(0), // init by 0.
            ctba_list: Vec::new(),
        }
    }
}

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
    /// get prdtl value from `CommandHeader.dw0`
    pub fn prdtl(&self) -> u32 {
        (self.dw0 >> 16) & 0xffff
    }
    pub fn w(&self) -> u32 {
        (self.dw0 >> 6) & 0x1
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
    dbc: u32,
}

impl PhysicalRegionDescriptor {
    /// Translate all dba to host physical address.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::uninit_vec,
        clippy::similar_names
    )]
    pub fn translate_data_base_address(
        &mut self,
        ctba_list: &mut Vec<CommandTableAddressData>,
        dir: &TransferDirection,
    ) {
        let db_gpa = GuestPhysicalAddress(((self.dbau as usize) << 32) | self.dba as usize);

        let data_base_size = self.dbc as usize + 1;
        if data_base_size <= PAGE_SIZE {
            let db_hpa = g_stage_trans_addr(db_gpa).expect("data base address translation failed");
            ctba_list.push(CommandTableAddressData::TranslatedAddress(db_gpa));
            self.dbau = ((db_hpa.raw() >> 32) & 0xffff_ffff) as u32;
            self.dba = (db_hpa.raw() & 0xffff_ffff) as u32;
        } else {
            crate::debugln!("[dbc] data base size: {:#x}", data_base_size);
            unsafe {
                let mut new_heap = Vec::<u8>::with_capacity(data_base_size);
                new_heap.set_len(data_base_size);
                let new_heap_addr = new_heap.as_ptr() as usize;

                self.dbau = ((new_heap_addr >> 32) & 0xffff_ffff) as u32;
                self.dba = (new_heap_addr & 0xffff_ffff) as u32;

                // write data to allocated memory if command is `write`
                if *dir == TransferDirection::HostToDevice {
                    let heap_ptr = new_heap.as_ptr().cast_mut();
                    for offset in (0..new_heap.len()).step_by(PAGE_SIZE) {
                        let dst_gpa = db_gpa + offset;
                        let dst_hpa = g_stage_trans_addr(dst_gpa)
                            .expect("failed translation of data base address");

                        core::ptr::copy(
                            dst_hpa.raw() as *const u8,
                            heap_ptr.add(offset) as *mut u8,
                            if offset + PAGE_SIZE < new_heap.len() {
                                PAGE_SIZE
                            } else {
                                new_heap.len() - offset
                            },
                        );
                    }
                }
                ctba_list.push(CommandTableAddressData::AllocatedAddress(db_gpa, new_heap));
            }
        }
    }

    /// Restore all dba to host physical address.
    #[allow(clippy::cast_possible_truncation, clippy::similar_names)]
    pub fn restore_data_base_address(
        &mut self,
        db_addr_data: &mut CommandTableAddressData,
        dir: &TransferDirection,
    ) {
        match db_addr_data {
            CommandTableAddressData::TranslatedAddress(db_gpa) => {
                self.dbau = ((db_gpa.raw() >> 32) & 0xffff_ffff) as u32;
                self.dba = (db_gpa.raw() & 0xffff_ffff) as u32;
            }
            CommandTableAddressData::AllocatedAddress(db_gpa, heap) => {
                self.dbau = ((db_gpa.raw() >> 32) & 0xffff_ffff) as u32;
                self.dba = (db_gpa.raw() & 0xffff_ffff) as u32;

                // write back data to guest memory if command is `read`
                if *dir == TransferDirection::DeviceToHost {
                    let heap_ptr = heap.as_ptr().cast_mut();
                    for offset in (0..heap.len()).step_by(PAGE_SIZE) {
                        let dst_gpa = *db_gpa + offset;
                        let dst_hpa = g_stage_trans_addr(dst_gpa)
                            .expect("failed translation of data base address");

                        unsafe {
                            core::ptr::copy(
                                heap_ptr.add(offset),
                                dst_hpa.raw() as *mut u8,
                                if offset + PAGE_SIZE < heap.len() {
                                    PAGE_SIZE
                                } else {
                                    heap.len() - offset
                                },
                            );
                        }
                    }
                }
                // free allocated region
                heap.clear();
            }
        }
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
    /// Translate all dba to host physical address.
    pub fn translate_all_data_base_addresses(
        &mut self,
        prdtl: u32,
        ctba_list: &mut Vec<CommandTableAddressData>,
        dir: &TransferDirection,
    ) {
        let prd_base_ptr = self.prdt.as_mut_ptr().cast::<PhysicalRegionDescriptor>();
        for index in 0..prdtl {
            unsafe {
                let prd_ptr = prd_base_ptr.add(index as usize);
                (*prd_ptr).translate_data_base_address(ctba_list, dir);
            }
        }
    }

    /// Restore all dba
    pub fn restore_all_data_base_addresses(
        &mut self,
        ctba_list: &mut [CommandTableAddressData],
        dir: &TransferDirection,
    ) {
        let prd_base_ptr = self.prdt.as_mut_ptr().cast::<PhysicalRegionDescriptor>();
        for (index, ctba) in ctba_list.iter_mut().enumerate() {
            unsafe {
                let prd_ptr = prd_base_ptr.add(index);
                (*prd_ptr).restore_data_base_address(ctba, dir);
            }
        }
    }
}
