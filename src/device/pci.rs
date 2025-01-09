//! PCI: Peripheral Component Interconnect

use super::{MmioDevice, PTE_FLAGS_FOR_DEVICE};
use crate::memmap::{GuestPhysicalAddress, HostPhysicalAddress, MemoryMap};
use fdt::Fdt;

use alloc::vec::Vec;

/// Pci device.
///
/// A struct that implement this trait **must** has `bus`, `device`, `function` number.
#[allow(clippy::module_name_repetitions)]
pub trait PciDevice {
    /// Create self instance.
    /// * `device_tree` - struct Fdt
    /// * `node_path` - node path in fdt
    fn new(device_tree: &Fdt, node_path: &str) -> Option<Self>
    where
        Self: Sized;

    /// Initialize pci device.
    /// * `pci` - struct `Pci`
    fn init(&self, pci: &Pci);
}

/// Registers in Common configuration Space Header.
///
/// Ref: [https://astralvx.com/storage/2020/11/PCI_Express_Base_4.0_Rev0.3_February19-2014.pdf](https://astralvx.com/storage/2020/11/PCI_Express_Base_4.0_Rev0.3_February19-2014.pdf) p. 578  
/// Ref: [https://osdev.jp/wiki/PCI-Memo](https://osdev.jp/wiki/PCI-Memo)  
/// Ref: [http://oswiki.osask.jp/?PCI](http://oswiki.osask.jp/?PCI)  
#[derive(Clone, Copy)]
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
    /// Base address of memory map.
    base_addr: HostPhysicalAddress,
    /// Memory map size.
    size: usize,
    /// Memory maps for pci devices
    memory_maps: Vec<MemoryMap>,
}

impl Pci {
    /// Read config data from "PCI Configuration Space".
    #[allow(clippy::cast_possible_truncation)]
    pub fn read_config_register(
        &self,
        bus_num: u32,
        device_num: u32,
        function_num: u32,
        reg: ConfigSpaceRegister,
    ) -> u32 {
        let config_data_reg_addr = self.base_addr.0 as u32
            | ((bus_num & 0b1111_1111) << 20)
            | ((device_num & 0b1_1111) << 15)
            | ((function_num & 0b111) << 12)
            | reg as u32;

        match reg {
            ConfigSpaceRegister::VendorId
            | ConfigSpaceRegister::DeviceId
            | ConfigSpaceRegister::Command
            | ConfigSpaceRegister::Status => unsafe {
                u32::from(core::ptr::read_volatile(config_data_reg_addr as *const u16))
            },
            ConfigSpaceRegister::BaseAddressRegister1
            | ConfigSpaceRegister::BaseAddressRegister2 => unsafe {
                core::ptr::read_volatile(config_data_reg_addr as *const u32)
            },
        }
    }

    /// Read config data from "PCI Configuration Space".
    #[allow(clippy::cast_possible_truncation)]
    pub fn write_config_register(
        &self,
        bus_num: u32,
        device_num: u32,
        function_num: u32,
        reg: ConfigSpaceRegister,
        data: u32,
    ) {
        let config_data_reg_addr = self.base_addr.0 as u32
            | ((bus_num & 0b1111_1111) << 20)
            | ((device_num & 0b1_1111) << 15)
            | ((function_num & 0b111) << 12)
            | reg as u32;
        match reg {
            ConfigSpaceRegister::VendorId
            | ConfigSpaceRegister::DeviceId
            | ConfigSpaceRegister::Command
            | ConfigSpaceRegister::Status => unsafe {
                core::ptr::write_volatile(config_data_reg_addr as *mut u16, data as u16);
            },
            ConfigSpaceRegister::BaseAddressRegister1
            | ConfigSpaceRegister::BaseAddressRegister2 => unsafe {
                core::ptr::write_volatile(config_data_reg_addr as *mut u32, data);
            },
        }
    }

    /// Return memory maps of Generic PCI host controller
    ///
    /// Ref: [https://www.kernel.org/doc/Documentation/devicetree/bindings/pci/host-generic-pci.txt](https://www.kernel.org/doc/Documentation/devicetree/bindings/pci/host-generic-pci.txt)
    pub fn pci_memory_maps(&self) -> &[MemoryMap] {
        &self.memory_maps
    }
}

impl MmioDevice for Pci {
    fn new(device_tree: &Fdt, node_path: &str) -> Self {
        /// Bytes size of u32.
        const BYTES_U32: usize = 4;
        /// Number of bytes in each range chunks.
        /// `BUS_ADDRESS(3)` - `CPU_PHYSICAL(2)` - `SIZE(2)`
        const RANGE_NUM: usize = 7;

        let region = device_tree
            .find_node(node_path)
            .unwrap()
            .reg()
            .unwrap()
            .next()
            .unwrap();
        let ranges = device_tree
            .find_node(node_path)
            .unwrap()
            .property("ranges")
            .unwrap()
            .value;

        assert!(ranges.len() % 4 == 0);
        assert!((ranges.len() / 4) % 7 == 0);

        let get_u32 = |range: &[u8], four_bytes_index: usize| {
            let index = four_bytes_index * 4;
            (u32::from(range[index]) << 24)
                | (u32::from(range[index + 1]) << 16)
                | (u32::from(range[index + 2]) << 8)
                | u32::from(range[index + 3])
        };
        let mut memory_maps = Vec::new();
        for range in ranges.chunks(RANGE_NUM * BYTES_U32) {
            let bus_address = get_u32(range, 0);

            // ignore I/O space map
            // https://elinux.org/Device_Tree_Usage#PCI_Address_Translation
            if (bus_address >> 24) & 0b11 != 0b01 {
                let address = ((get_u32(range, 3) as usize) << 32) | get_u32(range, 4) as usize;
                let size = ((get_u32(range, 5) as usize) << 32) | get_u32(range, 6) as usize;

                memory_maps.push(MemoryMap::new(
                    GuestPhysicalAddress(address)..GuestPhysicalAddress(address) + size,
                    HostPhysicalAddress(address)..HostPhysicalAddress(address) + size,
                    &PTE_FLAGS_FOR_DEVICE,
                ));
            }
        }

        Pci {
            base_addr: HostPhysicalAddress(region.starting_address as usize),
            size: region.size.unwrap(),
            memory_maps,
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
