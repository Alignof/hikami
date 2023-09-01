use fdt::Fdt;

pub const DRAM_BASE: usize = 0x8000_0000;
pub const PAGE_TABLE_BASE: usize = 0x8020_0000;
pub const PAGE_TABLE_SIZE: usize = 1024;
pub const STACK_BASE: usize = 0x8030_0000;
pub const STACK_SIZE_PER_HART: usize = 0x1_0000;
pub const PA2VA_OFFSET: usize = 0xffff_ffff_4000_0000;

pub struct Memmap {
    pub uart_addr: u64,
    pub initrd_addr: usize,
}

impl Memmap {
    pub fn new(device_tree: Fdt) -> Self {
        let uart_addr = device_tree
            .find_node("/soc/serial")
            .unwrap()
            .reg()
            .unwrap()
            .next()
            .unwrap()
            .starting_address as u64;
        let initrd_addr = device_tree
            .find_node("/soc/chosen")
            .unwrap()
            .property("linux,initrd-start")
            .unwrap()
            .value[0] as usize;

        Memmap {
            uart_addr,
            initrd_addr,
        }
    }
}
