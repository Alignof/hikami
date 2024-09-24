//! PCI: Peripheral Component Interconnect

use super::{MmioDevice, PTE_FLAGS_FOR_DEVICE};
use crate::memmap::{GuestPhysicalAddress, HostPhysicalAddress, MemoryMap};
use fdt::Fdt;

/// Registers in Common configuration Space Header.
///
/// Ref: [https://astralvx.com/storage/2020/11/PCI_Express_Base_4.0_Rev0.3_February19-2014.pdf](https://astralvx.com/storage/2020/11/PCI_Express_Base_4.0_Rev0.3_February19-2014.pdf) p. 578  
/// Ref: [https://osdev.jp/wiki/PCI-Memo](https://osdev.jp/wiki/PCI-Memo)  
/// Ref: [http://oswiki.osask.jp/?PCI](http://oswiki.osask.jp/?PCI)  
pub enum ConfigSpaceRegister {
    /// Vendor ID
    VendorId = 0x0,
    /// Device ID
    DeviceId = 0x2,
    /// Command
    Command = 0x4,
    /// Status
    Status = 0x6,
    /// Base Address Register 1
    BaseAddressRegister1 = 0x10,
    /// Base Address Register 2
    BaseAddressRegister2 = 0x14,
}

/// PCI: Peripheral Component Interconnect
/// Local computer bus.
#[derive(Debug)]
pub struct Pci {
    base_addr: HostPhysicalAddress,
    size: usize,
}

impl Pci {
    /// Read config data from "PCI Configuration Space".
    pub fn read_config_data(
        &self,
        bus_num: u32,
        device_num: u32,
        function_num: u32,
        reg: ConfigSpaceRegister,
    ) -> u32 {
        let config_data_reg_addr = self.base_addr.0 as u32
            | (bus_num & 0b1111_1111) << 20
            | (device_num & 0b1_1111) << 15
            | (function_num & 0b111) << 12
            | reg as u32;
        let config_data_reg_ptr = config_data_reg_addr as *const u32;

        unsafe { config_data_reg_ptr.read_volatile() }
    }

    /// Read config data from "PCI Configuration Space".
    pub fn write_config_data(
        &self,
        bus_num: u32,
        device_num: u32,
        function_num: u32,
        reg: ConfigSpaceRegister,
        data: u32,
    ) {
        let config_data_reg_addr = self.base_addr.0 as u32
            | (bus_num & 0b1111_1111) << 20
            | (device_num & 0b1_1111) << 15
            | (function_num & 0b111) << 12
            | reg as u32;
        let config_data_reg_ptr = config_data_reg_addr as *mut u32;

        unsafe {
            config_data_reg_ptr.write_volatile(data);
        }
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
