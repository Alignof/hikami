use super::Device;
use crate::memmap::page_table::PteFlag;
use crate::memmap::{constant, MemoryMap};
use fdt::Fdt;

const DEVICE_FLAGS: [PteFlag; 5] = [
    PteFlag::Dirty,
    PteFlag::Accessed,
    PteFlag::Write,
    PteFlag::Read,
    PteFlag::Valid,
];

// unused constant for now
// pub const ENABLE_BASE: usize = 0x2000;
// pub const ENABLE_PER_HART: usize = 0x80;
// pub const CONTEXT_BASE: usize = 0x20_0000;
// pub const CONTEXT_PER_HART: usize = 0x1000;
// pub const CONTEXT_CLAIM: usize = 0x4;

/// PLIC: Platform-Level Interrupt Controller  
/// Interrupt controller for global interrupts.
#[derive(Debug)]
pub struct Plic {
    base_addr: usize,
    size: usize,
}

impl Device for Plic {
    fn new(device_tree: &Fdt, node_path: &str) -> Self {
        let region = device_tree
            .find_node(node_path)
            .unwrap()
            .reg()
            .unwrap()
            .next()
            .unwrap();

        Plic {
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

    fn memmap(&self) -> MemoryMap {
        MemoryMap::new(
            self.vaddr()..self.vaddr() + self.size(),
            self.paddr()..self.paddr() + self.size(),
            &DEVICE_FLAGS,
        )
    }
}
