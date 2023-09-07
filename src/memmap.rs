pub mod constant;
use fdt::Fdt;


pub struct Device {
    addr: usize,
    size: usize,
}

impl Device {
    fn new(device_tree: &Fdt, node_path: &str) -> Self {
        let region = device_tree
            .find_node(node_path)
            .unwrap()
            .reg()
            .unwrap()
            .nth(0)
            .unwrap();

        Device {
            addr: region.starting_address as usize,
            size: region.size.unwrap(),
        }
    }

    fn new_by_property(
        device_tree: &Fdt,
        node_path: &str,
        start_prop: &str,
        end_prop: &str,
    ) -> Self {
        let node = device_tree.find_node(node_path).unwrap();
        let start = node.property(start_prop).unwrap().value[0] as usize;
        let end = node.property(end_prop).unwrap().value[0] as usize;

        Device {
            addr: start,
            size: end - start,
        }
    }

    fn paddr(&self) -> usize {
        self.addr
    }

    pub fn vaddr(&self) -> usize {
        self.addr + constant::PA2VA_DEVICE_OFFSET
    }
}

pub struct Memmap {
    pub uart: Device,
    pub initrd: Device,
    pub plic: Device,
}

impl Memmap {
    pub fn new(device_tree: Fdt) -> Self {
        Memmap {
            uart: Device::new(&device_tree, "/soc/serial"),
            initrd: Device::new_by_property(
                &device_tree,
                "/chosen",
                "linux,initrd-start",
                "linux,initrd-end",
            ),
            plic: Device::new(&device_tree, "/soc/plic"),
        }
    }
}
