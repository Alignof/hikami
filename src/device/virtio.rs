use super::{Device, PTE_FLAGS_FOR_DEVICE};
use crate::memmap::{GuestPhysicalAddress, HostPhysicalAddress, MemoryMap};
use alloc::vec::Vec;
use fdt::Fdt;

/// A virtualization standard for network and disk device drivers.
/// Since more than one may be found, we will temporarily use the first one.
#[derive(Debug)]
pub struct VirtIO {
    base_addr: HostPhysicalAddress,
    size: usize,
    irq: u8,
}

impl VirtIO {
    /// Create each Virt IO data when device has multiple IOs.
    pub fn new_all(device_tree: &Fdt, node_path: &str) -> Vec<Self> {
        device_tree
            .find_all_nodes(node_path)
            .map(|node| {
                let region = node.reg().unwrap().next().unwrap();
                let irq = node.property("interrupts").unwrap().value[0];
                VirtIO {
                    base_addr: HostPhysicalAddress(region.starting_address as usize),
                    size: region.size.unwrap(),
                    irq,
                }
            })
            .collect()
    }

    pub fn irq(&self) -> u8 {
        self.irq
    }
}

impl Device for VirtIO {
    fn new(device_tree: &Fdt, node_path: &str) -> Self {
        let node = device_tree.find_all_nodes(node_path).next().unwrap();
        let region = node.reg().unwrap().next().unwrap();
        let irq = node.property("interrupts").unwrap().value[0];

        VirtIO {
            base_addr: HostPhysicalAddress(region.starting_address as usize),
            size: region.size.unwrap(),
            irq,
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
