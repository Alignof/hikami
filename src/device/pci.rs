//! PCI: Peripheral Component Interconnect

// PCI devices
pub mod iommu;

pub mod config_register;

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
    fn new(bus: u32, device: u32, function: u32) -> Self;

    /// Initialize pci device.
    /// * `pci`: struct `Pci`
    fn init(&self, pci_config_space_base_addr: HostPhysicalAddress);
}

#[derive(Debug)]
/// Pci devices
struct PciDevices {
    /// IOMMU: I/O memory management unit.
    iommu: Option<iommu::IoMmu>,
}

impl PciDevices {
    pub fn new(device_tree: &Fdt, node_path: &str, memory_maps: &mut Vec<MemoryMap>) -> Self {
        /// Bytes size of u32.
        const BYTES_U32: usize = 4;
        /// Number of bytes in each range chunks.
        /// `BUS_ADDRESS(3)` - `CPU_PHYSICAL(2)` - `SIZE(2)`
        const RANGE_NUM: usize = 7;

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

        PciDevices {
            iommu: iommu::IoMmu::new_from_dtb(&device_tree, "soc/pci/iommu"),
        }
    }
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
    /// PCI devices
    pci_devices: PciDevices,
}

impl Pci {
    /// Return memory maps of Generic PCI host controller
    ///
    /// Ref: [https://www.kernel.org/doc/Documentation/devicetree/bindings/pci/host-generic-pci.txt](https://www.kernel.org/doc/Documentation/devicetree/bindings/pci/host-generic-pci.txt)
    pub fn pci_memory_maps(&self) -> &[MemoryMap] {
        &self.memory_maps
    }

    /// Initialize PCI devices.
    pub fn init_pci_devices(&self) {
        if let Some(iommu) = &self.pci_devices.iommu {
            iommu.init(self.base_addr);
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

        let mut memory_maps = Vec::new();
        let pci_devices = PciDevices::new(device_tree, node_path, &mut memory_maps);

        Pci {
            base_addr: HostPhysicalAddress(region.starting_address as usize),
            size: region.size.unwrap(),
            memory_maps,
            pci_devices,
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
