//! Serial ATA
//!
//! Ref: [https://osdev.jp/wiki/AHCI-Memo](https://osdev.jp/wiki/AHCI-Memo)

use super::config_register::{get_bar_size, read_config_register, ConfigSpaceHeaderField};
use super::{Bdf, PciAddressSpace, PciDevice};
use crate::device::DeviceEmulateError;
use crate::memmap::page_table::g_stage_trans_addr;
use crate::memmap::{GuestPhysicalAddress, HostPhysicalAddress, MemoryMap};

use alloc::vec::Vec;
use core::ops::Range;

/// SATA: Serial ATA
#[derive(Debug)]
pub struct Sata {
    /// Bus - device - function
    ident: Bdf,
    /// AHCI Base Address Register
    abar: Range<HostPhysicalAddress>,
    /// PCI Vender ID
    vender_id: u32,
    /// PCI Device ID
    device_id: u32,
}

impl Sata {
    /// Pass through loading memory
    fn pass_through_loading(&self, dst_addr: HostPhysicalAddress) -> u32 {
        let dst_ptr = dst_addr.raw() as *const u32;
        crate::println!("[ read] {:#x} -> {:#x}", dst_addr.0, unsafe {
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

        Ok(self.pass_through_loading(dst_addr))
    }

    /// Pass through storing memory
    fn pass_through_storing(&self, dst_addr: HostPhysicalAddress, value: u32) {
        let dst_ptr = dst_addr.raw() as *mut u32;
        crate::println!("[write] {:#x} <- {:#x}", dst_addr.0, value);
        unsafe {
            dst_ptr.write_volatile(value);
        }
    }

    /// Emulate storing base address to `CLB` of `FB`
    fn storing_base_addr(
        &mut self,
        dst_addr: HostPhysicalAddress,
        offset: usize,
        port_offset: usize,
        value: u32,
    ) -> Result<(), DeviceEmulateError> {
        let base_addr = self.abar.start;
        let cmd_list_gpa = if port_offset == 0x0 {
            let upper_addr = unsafe {
                core::ptr::read_volatile((base_addr.raw() + offset + 0x4) as *const u32) as usize
            };
            GuestPhysicalAddress(upper_addr << 32 | value as usize)
        } else {
            let lower_addr = unsafe {
                core::ptr::read_volatile((base_addr.raw() + offset) as *const u32) as usize
            };
            GuestPhysicalAddress((value as usize) << 32 | lower_addr)
        };
        if (0x9000_0000..0xa000_0000).contains(&cmd_list_gpa.raw()) {
            if let Ok(cmd_list_hpa) = g_stage_trans_addr(cmd_list_gpa) {
                crate::println!(
                    "[translate] P{}CLB: {:#x}(GPA) -> {:#x}(HPA)",
                    (offset - 0x100) / 0x80,
                    cmd_list_gpa.raw(),
                    cmd_list_hpa.raw()
                );

                let lower_offset = offset & !0xb111;
                unsafe {
                    core::ptr::write_volatile(
                        (base_addr.raw() + lower_offset) as *mut u32,
                        (cmd_list_hpa.raw() & 0xffff_ffff) as u32,
                    );
                    core::ptr::write_volatile(
                        (base_addr.raw() + lower_offset + 4) as *mut u32,
                        (cmd_list_hpa.raw() >> 32 & 0xffff_ffff) as u32,
                    );
                }

                return Ok(());
            }
        }

        // pass through as if it is in the middle of a translation
        self.pass_through_storing(dst_addr, value);

        Ok(())
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

        match offset {
            // 0x00 - 0x2b: Generic Host Control
            // 0x2c - 0x9f: Reserved
            // 0xa0 - 0xff: Vendor specific registers
            0x0..=0xff => self.pass_through_storing(dst_addr, value),
            // Port control registers
            0x100..=0x10ff => match offset % 0x80 {
                // 0x00: command list base address, 1K-byte aligned
                // 0x04: command list base address upper 32 bits
                port_offset @ (0x00 | 0x04) => {
                    self.storing_base_addr(dst_addr, offset, port_offset, value)?
                }
                // command issue
                0x38 => {
                    crate::println!("[command issue] {}", value.trailing_zeros());
                    crate::println!("[command issue] count one {}", value.count_ones());
                }
                // other registers
                _ => self.pass_through_storing(dst_addr, value),
            },
            _ => unreachable!("[HBA Memory Registers] out of range"),
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
            HostPhysicalAddress((bar_value & 0xfffffff0) as usize)
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
            ident: bdf,
            abar,
            vender_id,
            device_id,
        }
    }

    fn init(&self, _: HostPhysicalAddress) {
        unreachable!();
    }
}
