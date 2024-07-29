use super::Device;
use crate::memmap::page_table::PteFlag;
use crate::memmap::{constant, MemoryMap};
use fdt::Fdt;

const DEVICE_FLAGS: [PteFlag; 5] = [
    PteFlag::Dirty,
    PteFlag::Accessed,
    PteFlag::Write,
    PteFlag::Read,
    PteFlag::Valid,
];

/// A scheme for loading a temporary root file system into memory,
/// to be used as part of the Linux startup process.
#[derive(Debug)]
pub struct Initrd {
    base_addr: usize,
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
            base_addr: start,
            size: end - start,
        }
    }

    fn size(&self) -> usize {
        self.size
    }

    fn paddr(&self) -> usize {
        self.base_addr
    }

    fn vaddr(&self) -> usize {
        self.base_addr + constant::PA2VA_DEVICE_OFFSET
    }

    fn memmap(&self) -> MemoryMap {
        MemoryMap::new(
            self.vaddr()..self.vaddr() + self.size(),
            self.paddr()..self.paddr() + self.size(),
            &DEVICE_FLAGS,
        )
    }

    fn identity_memmap(&self) -> MemoryMap {
        MemoryMap::new(
            self.paddr()..self.paddr() + self.size(),
            self.paddr()..self.paddr() + self.size(),
            &DEVICE_FLAGS,
        )
    }
}
