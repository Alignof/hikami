use fdt::Fdt;

pub mod constant {
    pub const DRAM_BASE: usize = 0x8000_0000;
    pub const DRAM_SIZE_PAR_HART: usize = 0x1000_0000;
    pub const PAGE_TABLE_BASE: usize = 0x8020_0000;
    pub const PAGE_SIZE: usize = 4096;
    pub const PAGE_TABLE_OFFSET_PER_HART: usize = 1024;
    pub const STACK_BASE: usize = 0x8030_0000;
    pub const STACK_SIZE_PER_HART: usize = 0x1_0000;
    pub const PA2VA_DRAM_OFFSET: usize = 0xffff_ffff_4000_0000;
    pub const PA2VA_DEVICE_OFFSET: usize = 0xffff_fffc_0000_0000;
}

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
