//! IOMMU: I/O memory management unit.
//! Ref: [https://github.com/riscv-non-isa/riscv-iommu/releases/download/v1.0.0/riscv-iommu.pdf](https://github.com/riscv-non-isa/riscv-iommu/releases/download/v1.0.0/riscv-iommu.pdf)

use super::{Device, PTE_FLAGS_FOR_DEVICE};
use crate::memmap::{GuestPhysicalAddress, HostPhysicalAddress, MemoryMap};
use fdt::Fdt;

/// IOMMU: I/O memory management unit.
#[derive(Debug)]
pub struct IoMmu {
    base_addr: HostPhysicalAddress,
    size: usize,
}

impl Device for IoMmu {
    fn new(device_tree: &Fdt, node_path: &str) -> Self {
        let region = device_tree
            .find_node(node_path)
            .unwrap()
            .reg()
            .unwrap()
            .next()
            .unwrap();

        IoMmu {
            base_addr: HostPhysicalAddress(region.starting_address as usize),
            size: region.size.unwrap(),
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
