//! Serial ATA
//!
//! Ref: [https://osdev.jp/wiki/AHCI-Memo](https://osdev.jp/wiki/AHCI-Memo)

use super::config_register::{get_bar_size, read_config_register, ConfigSpaceHeaderField};
use super::{Bdf, PciAddressSpace, PciDevice};
use crate::memmap::{HostPhysicalAddress, MemoryMap};

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
