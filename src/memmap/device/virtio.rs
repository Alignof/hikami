use super::Device;
use crate::memmap::constant;
use fdt::Fdt;

/// A virtualization standard for network and disk device drivers.
/// Since more than one may be found, we will temporarily use the first one.
pub struct VirtIO {
    base_addr: usize,
    size: usize,
}

impl VirtIO {
    pub fn new_all(device_tree: &Fdt, node_path: &str) -> Vec<Self> {
        device_tree
            .find_all_nodes(node_path)
            .map(|node| {
                let region = node.reg().unwrap().next().unwrap();
                VirtIO {
                    base_addr: region.starting_address as usize,
                    size: region.size.unwrap(),
                }
            })
            .collect()
    }
}

impl Device for VirtIO {
    fn new(device_tree: &Fdt, node_path: &str) -> Self {
        let region = device_tree
            .find_all_nodes(node_path)
            .next()
            .unwrap()
            .reg()
            .unwrap()
            .next()
            .unwrap();

        VirtIO {
            base_addr: region.starting_address as usize,
            size: region.size.unwrap(),
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
}
