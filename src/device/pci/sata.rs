//! Serial ATA
//!
//! Ref: [https://osdev.jp/wiki/AHCI-Memo](https://osdev.jp/wiki/AHCI-Memo)

mod command;

use super::config_register::{get_bar_size, read_config_register, ConfigSpaceHeaderField};
use super::{Bdf, PciAddressSpace, PciDevice};
use crate::device::DeviceEmulateError;
use crate::memmap::page_table::g_stage_trans_addr;
use crate::memmap::{GuestPhysicalAddress, HostPhysicalAddress, MemoryMap};
use command::{CommandHeader, CommandTable, CommandTableGpaStorage, COMMAND_HEADER_SIZE};

use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use core::ops::Range;

/// Number of SATA port.
const SATA_PORT_NUM: usize = 32;
/// Offset of port control registers.
const PORT_CONTROL_REGS_OFFSET: usize = 0x100;
/// Size of port control registers.
const PORT_CONTROL_REGS_SIZE: usize = 0x80;

/// HBA(Host Bus Adapter) Port
#[derive(Debug, Clone)]
struct HbaPort {
    /// Command list base address
    cmd_list_gpa: GuestPhysicalAddress,
    /// FIS base address
    fis_gpa: GuestPhysicalAddress,
    /// Commands status.
    ///
    /// It is copy of `Port x Command Issue`(0x38) at the time of writing.
    commands_status: u32,
    /// Addresses of `CommandTable` and its each CTBA.
    cmd_table_gpa_storage: [CommandTableGpaStorage; COMMAND_HEADER_SIZE],
}

impl HbaPort {
    /// Generate new `HbaPort`.
    pub const fn new() -> Self {
        HbaPort {
            cmd_list_gpa: GuestPhysicalAddress(0), // init by 0.
            fis_gpa: GuestPhysicalAddress(0),      // init by 0.
            commands_status: 0,
            cmd_table_gpa_storage: [const { CommandTableGpaStorage::new() }; COMMAND_HEADER_SIZE],
        }
    }

    /// Pass through loading memory
    fn pass_through_loading(dst_addr: HostPhysicalAddress) -> u32 {
        let dst_ptr = dst_addr.raw() as *const u32;
        unsafe { dst_ptr.read_volatile() }
    }

    /// Emulate loading port registers.
    #[allow(clippy::cast_possible_truncation)]
    pub fn emulate_loading(
        &self,
        base_addr: HostPhysicalAddress,
        dst_addr: HostPhysicalAddress,
    ) -> u32 {
        let offset = dst_addr.raw() - base_addr.raw();
        let port_offset = offset % PORT_CONTROL_REGS_SIZE;
        match port_offset {
            // 0x00: command list base address, 1K-byte aligned
            0x0 => (self.cmd_list_gpa.raw() & 0xffff_ffff) as u32,
            // 0x04: command list base address upper 32 bits
            0x4 => ((self.cmd_list_gpa.raw() >> 32) & 0xffff_ffff) as u32,
            // 0x08: FIS base address, 256-byte aligned
            0x8 => (self.fis_gpa.raw() & 0xffff_ffff) as u32,
            // 0x0c: FIS base address upper 32 bits
            0xc => ((self.fis_gpa.raw() >> 32) & 0xffff_ffff) as u32,
            // other registers
            _ => Self::pass_through_loading(dst_addr),
        }
    }

    /// Emulate storing base address to `CLB` of `FB`
    #[allow(clippy::cast_possible_truncation)]
    fn storing_base_addr(
        &mut self,
        hba_base_addr: HostPhysicalAddress,
        offset: usize,
        port_offset: usize,
        value: u32,
    ) {
        let base_gpa = if port_offset % 8 == 0 {
            let upper_addr = unsafe {
                core::ptr::read_volatile((hba_base_addr.raw() + offset + 0x4) as *const u32)
                    as usize
            };
            GuestPhysicalAddress((upper_addr << 32) | value as usize)
        } else {
            let lower_addr = unsafe {
                core::ptr::read_volatile((hba_base_addr.raw() + offset) as *const u32) as usize
            };
            GuestPhysicalAddress((value as usize) << 32 | lower_addr)
        };

        // store base guest physical addr
        if port_offset == 0x0 || port_offset == 0x4 {
            self.cmd_list_gpa = base_gpa;
        } else {
            self.fis_gpa = base_gpa;
        }

        if (0x9000_0000..0xa000_0000).contains(&base_gpa.raw()) {
            if let Ok(base_hpa) = g_stage_trans_addr(base_gpa) {
                if port_offset == 0x0 || port_offset == 0x4 {
                    crate::debugln!(
                        "[translate] P{}CLB: {:#x}(GPA) -> {:#x}(HPA)",
                        (offset - PORT_CONTROL_REGS_OFFSET) / PORT_CONTROL_REGS_SIZE,
                        base_gpa.raw(),
                        base_hpa.raw()
                    );
                } else {
                    crate::debugln!(
                        "[translate] P{}FB: {:#x}(GPA) -> {:#x}(HPA)",
                        (offset - PORT_CONTROL_REGS_OFFSET) / PORT_CONTROL_REGS_SIZE,
                        base_gpa.raw(),
                        base_hpa.raw()
                    );
                }

                let lower_offset = offset & !0b111;
                unsafe {
                    core::ptr::write_volatile(
                        (hba_base_addr.raw() + lower_offset) as *mut u32,
                        (base_hpa.raw() & 0xffff_ffff) as u32,
                    );
                    core::ptr::write_volatile(
                        (hba_base_addr.raw() + lower_offset + 4) as *mut u32,
                        ((base_hpa.raw() >> 32) & 0xffff_ffff) as u32,
                    );
                }
            }
        }
    }

    /// Rewrite address in command list and command table to host physical address.
    #[allow(clippy::cast_possible_truncation, clippy::similar_names)]
    fn rewrite_cmd_addr(&mut self, base_addr: HostPhysicalAddress, port_num: usize, cmd_num: u32) {
        let cmd_list_reg_addr =
            base_addr + PORT_CONTROL_REGS_OFFSET + PORT_CONTROL_REGS_SIZE * port_num;
        let cmd_list_hpa = unsafe {
            HostPhysicalAddress((cmd_list_reg_addr.0 as *const u32).read_volatile() as usize)
        };
        let cmd_header_hpa = cmd_list_hpa + cmd_num as usize * COMMAND_HEADER_SIZE;
        let cmd_header_ptr = cmd_header_hpa.raw() as *mut CommandHeader;
        let prdtl = unsafe { (*cmd_header_ptr).prdtl() };

        // translate command table address
        let cmd_table_gpa = unsafe {
            GuestPhysicalAddress(
                (((*cmd_header_ptr).ctba_u as usize) << 32) | (*cmd_header_ptr).ctba as usize,
            )
        };
        let cmd_table_hpa =
            g_stage_trans_addr(cmd_table_gpa).expect("command_header.ctba is not GPA");

        // store gpa
        self.cmd_table_gpa_storage[cmd_num as usize].cmd_table_gpa = cmd_table_gpa;

        // write command table host physical address
        unsafe {
            (*cmd_header_ptr).ctba_u = ((cmd_table_hpa.raw() >> 32) & 0xffff_ffff) as u32;
            (*cmd_header_ptr).ctba = (cmd_table_hpa.raw() & 0xffff_ffff) as u32;
        }

        let cmd_table_ptr = cmd_table_hpa.raw() as *mut CommandTable;
        unsafe {
            (*cmd_table_ptr).translate_all_data_base_addresses(
                prdtl,
                &mut self.cmd_table_gpa_storage[cmd_num as usize].ctba_list,
            );
        }
    }

    /// Restore address in command list and command table to `GuestPhysicalAddress` physical address.
    #[allow(clippy::cast_possible_truncation, clippy::similar_names)]
    fn restore_cmd_addr(&mut self, base_addr: HostPhysicalAddress, port_num: usize, cmd_num: u32) {
        let cmd_list_reg_addr =
            base_addr + PORT_CONTROL_REGS_OFFSET + PORT_CONTROL_REGS_SIZE * port_num;
        let cmd_list_hpa = unsafe {
            HostPhysicalAddress((cmd_list_reg_addr.0 as *const u32).read_volatile() as usize)
        };
        let cmd_header_hpa = cmd_list_hpa + cmd_num as usize * COMMAND_HEADER_SIZE;
        let cmd_header_ptr = cmd_header_hpa.raw() as *mut CommandHeader;

        // load hpa
        let cmd_table_hpa = unsafe {
            HostPhysicalAddress(
                (((*cmd_header_ptr).ctba_u as usize) << 32) | (*cmd_header_ptr).ctba as usize,
            )
        };

        // restore gpa
        let cmd_table_gpa = self.cmd_table_gpa_storage[cmd_num as usize].cmd_table_gpa;

        // write command table host physical address
        unsafe {
            (*cmd_header_ptr).ctba_u = ((cmd_table_gpa.raw() >> 32) & 0xffff_ffff) as u32;
            (*cmd_header_ptr).ctba = (cmd_table_gpa.raw() & 0xffff_ffff) as u32;
        }

        let cmd_table_ptr = cmd_table_hpa.raw() as *mut CommandTable;
        unsafe {
            (*cmd_table_ptr).restore_all_data_base_addresses(
                &mut self.cmd_table_gpa_storage[cmd_num as usize].ctba_list,
            );
        }

        self.cmd_table_gpa_storage[cmd_num as usize]
            .ctba_list
            .clear();
        self.cmd_table_gpa_storage[cmd_num as usize].cmd_table_gpa = GuestPhysicalAddress(0);
    }

    /// Pass through storing memory
    fn pass_through_storing(dst_addr: HostPhysicalAddress, value: u32) {
        let dst_ptr = dst_addr.raw() as *mut u32;
        unsafe {
            dst_ptr.write_volatile(value);
        }
    }

    /// Emulate storing port registers.
    pub fn emulate_storing(
        &mut self,
        base_addr: HostPhysicalAddress,
        dst_addr: HostPhysicalAddress,
        value: u32,
    ) {
        let offset = dst_addr.raw() - base_addr.raw();
        let port_offset = offset % PORT_CONTROL_REGS_SIZE;
        crate::debugln!(
            "[port{} write] {:#x} <- {:#x}",
            (offset - PORT_CONTROL_REGS_OFFSET) / PORT_CONTROL_REGS_SIZE,
            offset % PORT_CONTROL_REGS_SIZE,
            value
        );
        match port_offset {
            // 0x00: command list base address, 1K-byte aligned
            // 0x04: command list base address upper 32 bits
            // 0x08: FIS base address, 256-byte aligned
            // 0x0c: FIS base address upper 32 bits
            port_offset @ (0x00 | 0x04 | 0x08 | 0x0c) => {
                self.storing_base_addr(base_addr, offset, port_offset, value);
            }
            // interrupt status
            // Ref: https://osdev.jp/wiki/AHCI-Memo, Offset 10h: PxIS - Port Interrupt Status
            0x10 => {
                // command has already issued
                if self.commands_status != 0 {
                    // get completed command number
                    let current_cmd_status = Self::pass_through_loading(dst_addr + 0x28); // current command isssue value
                    let completed_cmd_num =
                        (self.commands_status & !current_cmd_status).trailing_zeros();
                    crate::debugln!("[command completed] {}", completed_cmd_num);

                    // restore translated address.
                    let port_num = (offset - PORT_CONTROL_REGS_OFFSET) / PORT_CONTROL_REGS_SIZE;
                    self.restore_cmd_addr(base_addr, port_num, completed_cmd_num);
                }

                Self::pass_through_storing(dst_addr, value);
            }
            // command issue
            0x38 => {
                let cmd_num = value.trailing_zeros();
                let port_num = (offset - PORT_CONTROL_REGS_OFFSET) / PORT_CONTROL_REGS_SIZE;
                crate::debugln!("[command issue] {}", cmd_num);
                self.rewrite_cmd_addr(base_addr, port_num, cmd_num);
                self.commands_status = Self::pass_through_loading(dst_addr) | value;

                Self::pass_through_storing(dst_addr, value);
            }
            // other registers
            _ => Self::pass_through_storing(dst_addr, value),
        }
    }
}

/// SATA: Serial ATA
#[derive(Debug)]
pub struct Sata {
    /// Bus - device - function
    _ident: Bdf,
    /// AHCI Base Address Register
    abar: Range<HostPhysicalAddress>,
    /// HBA Ports
    ports: Box<[HbaPort]>,
    /// PCI Vender ID
    _vender_id: u32,
    /// PCI Device ID
    _device_id: u32,
}

impl Sata {
    /// Pass through loading memory
    fn pass_through_loading(dst_addr: HostPhysicalAddress) -> u32 {
        let dst_ptr = dst_addr.raw() as *const u32;
        crate::debugln!("[ read] {:#x} -> {:#x}", dst_addr.0, unsafe {
            dst_ptr.read_volatile()
        });
        unsafe { dst_ptr.read_volatile() }
    }

    /// Emulate loading HBA Memory Registers.
    pub fn emulate_loading(
        &self,
        dst_addr: HostPhysicalAddress,
    ) -> Result<u32, DeviceEmulateError> {
        if !self.abar.contains(&dst_addr) {
            return Err(DeviceEmulateError::InvalidAddress);
        }

        let base_addr = self.abar.start;
        let offset = dst_addr.raw() - base_addr.raw();

        #[allow(clippy::match_same_arms)]
        match offset {
            // 0x00 - 0x2b: Generic Host Control
            // 0x2c - 0x9f: Reserved
            // 0xa0 - 0xff: Vendor specific registers
            0x0..=0xff => Ok(Self::pass_through_loading(dst_addr)),
            // Port control registers
            0x100..=0x10ff => {
                let port_num = (offset - PORT_CONTROL_REGS_OFFSET) / PORT_CONTROL_REGS_SIZE;
                let loaded_data = self.ports[port_num].emulate_loading(base_addr, dst_addr);
                crate::debugln!(
                    "[port{}  read] {:#x} -> {:#x}",
                    port_num,
                    offset % PORT_CONTROL_REGS_SIZE,
                    loaded_data
                );
                Ok(loaded_data)
            }
            // out of range but it may be used by others.
            _ => Ok(Self::pass_through_loading(dst_addr)),
        }
    }

    /// Pass through storing memory
    fn pass_through_storing(dst_addr: HostPhysicalAddress, value: u32) {
        let dst_ptr = dst_addr.raw() as *mut u32;
        crate::debugln!("[write] {:#x} <- {:#x}", dst_addr.0, value);
        unsafe {
            dst_ptr.write_volatile(value);
        }
    }

    /// Emulate storing HBA Memory Registers.
    pub fn emulate_storing(
        &mut self,
        dst_addr: HostPhysicalAddress,
        value: u32,
    ) -> Result<(), DeviceEmulateError> {
        if !self.abar.contains(&dst_addr) {
            return Err(DeviceEmulateError::InvalidAddress);
        }

        let base_addr = self.abar.start;
        let offset = dst_addr.raw() - base_addr.raw();

        #[allow(clippy::match_same_arms)]
        match offset {
            // 0x00 - 0x2b: Generic Host Control
            // 0x2c - 0x9f: Reserved
            // 0xa0 - 0xff: Vendor specific registers
            0x0..=0xff => Self::pass_through_storing(dst_addr, value),
            // Port control registers
            0x100..=0x10ff => {
                let port_num = (offset - PORT_CONTROL_REGS_OFFSET) / PORT_CONTROL_REGS_SIZE;
                self.ports[port_num].emulate_storing(base_addr, dst_addr, value);
            }
            // out of range but it may be used by others.
            _ => Self::pass_through_storing(dst_addr, value),
        }

        Ok(())
    }
}

impl PciDevice for Sata {
    fn new(
        bdf: Bdf,
        vender_id: u32,
        device_id: u32,
        pci_config_space_base_addr: HostPhysicalAddress,
        pci_addr_space: &PciAddressSpace,
        _memory_maps: &mut Vec<MemoryMap>,
    ) -> Self {
        let config_space_header_addr =
            pci_config_space_base_addr.0 | bdf.calc_config_space_header_offset();

        let bar_value = read_config_register(
            config_space_header_addr,
            ConfigSpaceHeaderField::BaseAddressRegister5,
        );

        // memory map
        assert_eq!(bar_value & 0x1, 0x0);
        let start_address = if bar_value == 0 {
            pci_addr_space.base_addr
        } else {
            HostPhysicalAddress((bar_value & 0xffff_fff0) as usize)
        };
        let size = get_bar_size(
            config_space_header_addr,
            ConfigSpaceHeaderField::BaseAddressRegister5,
        );
        let abar = Range {
            start: start_address,
            end: start_address + size as usize,
        };

        Sata {
            _ident: bdf,
            abar,
            ports: vec![HbaPort::new(); SATA_PORT_NUM].into_boxed_slice(),
            _vender_id: vender_id,
            _device_id: device_id,
        }
    }

    fn init(&self, _: HostPhysicalAddress) {
        unreachable!();
    }
}
