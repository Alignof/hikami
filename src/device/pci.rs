//! PCI: Peripheral Component Interconnect

use super::{MmioDevice, PTE_FLAGS_FOR_DEVICE};
use crate::memmap::{GuestPhysicalAddress, HostPhysicalAddress, MemoryMap};
use fdt::Fdt;

/// PCI: Peripheral Component Interconnect
/// Local computer bus.
#[derive(Debug)]
pub struct Pci {
    base_addr: HostPhysicalAddress,
    size: usize,
}

impl Pci {
    /// Set `CONFIG_ADDRESS` register.
    fn set_config_address(&self, bus_num: u32, device_num: u32, function_num: u32, offset: u32) {
        let config_addr_reg_ptr = self.base_addr.0 as *mut u32;
        let enable_bit = 1 << 31;

        unsafe {
            config_addr_reg_ptr.write_volatile(
                enable_bit
                    | (bus_num & 0b1111_1111) << 16
                    | (device_num & 0b1_1111) << 11
                    | (function_num & 0b111) << 8
                    | offset,
            );
        }
    }

    /// Unset `CONFIG_ADDRESS` register.
    fn unset_config_address(&self, bus_num: u32, device_num: u32, function_num: u32, offset: u32) {
        let config_addr_reg_ptr = self.base_addr.0 as *mut u32;

        unsafe {
            config_addr_reg_ptr.write_volatile(
                (bus_num & 0b1111_1111) << 16
                    | (device_num & 0b1_1111) << 11
                    | (function_num & 0b111) << 8
                    | offset,
            );
        }
    }

    /// Read config data from "PCI Configuration Space".
    pub fn read_config_data(
        &self,
        bus_num: u32,
        device_num: u32,
        function_num: u32,
        offset: u32,
    ) -> u32 {
        let config_data_reg_addr = self.base_addr.0 as u32
            | (bus_num & 0b1111_1111) << 20
            | (device_num & 0b1_1111) << 15
            | (function_num & 0b111) << 12
            | offset;
        let config_data_reg_ptr = config_data_reg_addr as *const u32;

        unsafe { config_data_reg_ptr.read_volatile() }
    }
}

impl MmioDevice for Pci {
    fn new(device_tree: &Fdt, node_path: &str) -> Self {
        let region = device_tree
            .find_node(node_path)
            .unwrap()
            .reg()
            .unwrap()
            .next()
            .unwrap();

        Pci {
            base_addr: HostPhysicalAddress(region.starting_address as usize),
            size: region.size.unwrap(),
        }
    }

    fn size(&self) -> usize {
        self.size
    }

    fn paddr(&self) -> HostPhysicalAddress {
        self.base_addr
    }

    fn memmap(&self) -> MemoryMap {
        let vaddr = GuestPhysicalAddress(self.paddr().raw());
        MemoryMap::new(
            vaddr..vaddr + self.size(),
            self.paddr()..self.paddr() + self.size(),
            &PTE_FLAGS_FOR_DEVICE,
        )
    }
}
