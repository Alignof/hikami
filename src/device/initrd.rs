use super::{Device, PTE_FLAGS_FOR_DEVICE};
use crate::memmap::{GuestPhysicalAddress, HostPhysicalAddress, MemoryMap};
use fdt::Fdt;

/// A scheme for loading a temporary root file system into memory,
/// to be used as part of the Linux startup process.
#[derive(Debug)]
pub struct Initrd {
    base_addr: HostPhysicalAddress,
    size: usize,
}

impl Device for Initrd {
    fn new(device_tree: &Fdt, node_path: &str) -> Self {
        let start_prop = "linux,initrd-start";
        let end_prop = "linux,initrd-end";
        let node = device_tree.find_node(node_path).unwrap();
        let start = node.property(start_prop).unwrap().value;
        let start = u32::from_be_bytes(start.try_into().unwrap()) as usize;
        let end = node.property(end_prop).unwrap().value;
        let end = u32::from_be_bytes(end.try_into().unwrap()) as usize;

        Initrd {
            base_addr: HostPhysicalAddress(start),
            size: end - start,
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
