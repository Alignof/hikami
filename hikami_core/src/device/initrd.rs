//! initrd: INITial RamDisk
#![allow(clippy::doc_markdown)]

use super::MmioDevice;
use crate::memmap::HostPhysicalAddress;

use alloc::string::{String, ToString};
use fdt::{Fdt, standard_nodes::MemoryRegion};

/// A scheme for loading a temporary root file system into memory,
/// to be used as part of the Linux startup process.
#[derive(Debug)]
pub struct Initrd {
    /// Device tree name
    name: String,
    /// Memory mapped register region
    memory_region: MemoryRegion,
}

impl Initrd {
    /// Try to get Initrd data and return the `Initrd`.
    pub fn try_new_from_node_path(
        root_page_table_addr: HostPhysicalAddress,
        device_tree: &Fdt,
        node_path: &str,
    ) -> Option<Self> {
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
                let memory_region = MemoryRegion {
                    starting_address: start as *const u8,
                    size: Some(end - start),
                };

                Self::create_page_table(root_page_table_addr, &[memory_region], node.name);

                Some(Initrd {
                    name: "linux,initrd".to_string(),
                    memory_region,
                })
            }
            None => None,
        }
    }
}

impl MmioDevice for Initrd {
    fn try_new(
        _root_page_table_addr: HostPhysicalAddress,
        _device_tree: &Fdt,
        _compatibles: &[&str],
    ) -> Option<Self> {
        unreachable!("use Initrd::try_new_from_node_path instead")
    }

    fn name(&self) -> &str {
        &self.name
    }
}
