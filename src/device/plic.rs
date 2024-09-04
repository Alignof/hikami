//! PLIC: Platform-Level Interrupt Controller  
//! ref: [https://github.com/riscv/riscv-plic-spec/releases/download/1.0.0/riscv-plic-1.0.0.pdf](https://github.com/riscv/riscv-plic-spec/releases/download/1.0.0/riscv-plic-1.0.0.pdf)

use super::{Device, PTE_FLAGS_FOR_DEVICE};
use crate::memmap::constant::MAX_HART_NUM;
use crate::memmap::{GuestPhysicalAddress, HostPhysicalAddress, MemoryMap};
use fdt::Fdt;

// unused constant for now
// pub const ENABLE_BASE: usize = 0x2000;
// pub const ENABLE_PER_HART: usize = 0x80;
// pub const CONTEXT_CLAIM: usize = 0x4;
const CONTEXT_BASE: usize = 0x20_0000;
const CONTEXT_PER_HART: usize = 0x1000;

/// PLIC emulation result.
pub enum PlicEmulateError {
    InvalidAddress,
}

/// PLIC: Platform-Level Interrupt Controller  
/// Interrupt controller for global interrupts.
#[derive(Debug)]
pub struct Plic {
    base_addr: HostPhysicalAddress,
    size: usize,
    claim_complete: [u32; MAX_HART_NUM],
}

impl Plic {
    pub fn emulate_read(&self, dst_addr: HostPhysicalAddress) -> Result<usize, PlicEmulateError> {
        let offset = self.base_addr.raw() - dst_addr.raw();
        if offset < CONTEXT_BASE || offset > CONTEXT_BASE + CONTEXT_PER_HART * MAX_HART_NUM {
            return Err(PlicEmulateError::InvalidAddress);
        }

        let hart = (offset - CONTEXT_BASE) / CONTEXT_PER_HART;
        Ok(self.claim_complete[hart] as usize)
    }
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
            base_addr: HostPhysicalAddress(region.starting_address as usize),
            size: region.size.unwrap(),
            claim_complete: [0u32; MAX_HART_NUM],
        }
    }

    fn size(&self) -> usize {
        self.size
    }

    fn paddr(&self) -> HostPhysicalAddress {
        self.base_addr
    }

    fn memmap(&self) -> MemoryMap {
        let vaddr = GuestPhysicalAddress(self.paddr().raw());
        MemoryMap::new(
            vaddr..vaddr + self.size(),
            self.paddr()..self.paddr() + self.size(),
            &PTE_FLAGS_FOR_DEVICE,
        )
    }
}
