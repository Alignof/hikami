use fdt::Fdt;

pub const DRAM_BASE: u64 = 0x8000_0000;
pub const PAGE_TABLE_BASE: u64 = 0x8020_0000;
pub const PAGE_TABLE_SIZE: u64 = 1024;
pub const STACK_BASE: u64 = 0x8030_0000;
pub const PA2VA_OFFSET: u64 = 0xffff_ffff_4000_0000;

pub struct Memmap {
    pub uart_addr: u64,
}

impl Memmap {
    pub fn new(device_tree: Fdt) -> Self {
        let uart_addr = device_tree
            .find_node("/soc/uart")
            .unwrap()
            .reg()
            .unwrap()
            .next()
            .unwrap()
            .starting_address as u64;

        Memmap { uart_addr }
    }
}
