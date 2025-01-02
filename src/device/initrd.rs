//! initrd: INITial RamDisk
#![allow(clippy::doc_markdown)]

use super::{MmioDevice, PTE_FLAGS_FOR_DEVICE};
use crate::memmap::{GuestPhysicalAddress, HostPhysicalAddress, MemoryMap};
use fdt::Fdt;

/// A scheme for loading a temporary root file system into memory,
/// to be used as part of the Linux startup process.
#[derive(Debug)]
pub struct Initrd {
    /// Base address of memory map.
    base_addr: HostPhysicalAddress,
    /// Memory map size.
    size: usize,
}

impl Initrd {
    pub fn try_new(device_tree: &Fdt, node_path: &str) -> Option<Self> {
        let start_prop = "linux,initrd-start";
        let end_prop = "linux,initrd-end";
        let node = device_tree.find_node(node_path).unwrap();

        // linux,initrd-start = <0x00 0xa0000000> -> [0, 0, 0, 0, 160, 0, 0, 0]
        // `start[4..]` means skipping first four bytes.
        match node.property(start_prop) {
            Some(start) => {
                let start = start.value;
                let start = u32::from_be_bytes(start[4..].try_into().unwrap()) as usize;
                let end = node.property(end_prop).unwrap().value;
                let end = u32::from_be_bytes(end[4..].try_into().unwrap()) as usize;

                Some(Initrd {
                    base_addr: HostPhysicalAddress(start),
                    size: end - start,
                })
            }
            None => None,
        }
    }
}

impl MmioDevice for Initrd {
    fn new(_device_tree: &Fdt, _node_path: &str) -> Self {
        panic!("use Initrd::try_new instead");
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
