use fdt::Fdt;

pub const DRAM_BASE: usize = 0x8000_0000;
pub const PAGE_TABLE_BASE: usize = 0x8020_0000;
pub const PAGE_TABLE_SIZE: usize = 1024;
pub const STACK_BASE: usize = 0x8030_0000;
pub const STACK_SIZE_PER_HART: usize = 0x1_0000;
pub const PA2VA_OFFSET: usize = 0xffff_ffff_4000_0000;

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
}

pub struct Memmap {
    pub uart: Device,
    pub initrd: Device,
}

impl Memmap {
    pub fn new(device_tree: Fdt) -> Self {
        let uart = Device::new(&device_tree, "/soc/serial");
        let initrd = Device::new_by_property(
            &device_tree,
            "/chosen",
            "linux,initrd-start",
            "linux,initrd-end",
        );

        Memmap { uart, initrd }
    }
}
