use super::Device;
use crate::memmap::constant;
use fdt::Fdt;

pub struct VirtIO {
    base_addr: usize,
    size: usize,
}

impl Device for VirtIO {
    fn new(device_tree: &Fdt, node_path: &str) -> Self {
        let region = device_tree
            .find_all_nodes(node_path)
            .next()
            .unwrap()
            .reg()
            .unwrap()
            .nth(0)
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
