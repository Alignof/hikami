use super::Device;
use crate::memmap::constant;
use fdt::Fdt;

pub struct Initrd {
    base_addr: usize,
    size: usize,
}

impl Device for Initrd {
    fn new(device_tree: &Fdt, node_path: &str) -> Self {
        let start_prop = "linux-initrd-start";
        let end_prop = "linux-initrd-end";
        let node = device_tree.find_node(node_path).unwrap();
        let start = node.property(start_prop).unwrap().value[0] as usize;
        let end = node.property(end_prop).unwrap().value[0] as usize;

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
}
