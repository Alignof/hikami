//! CLINT: *C*ore *L*ocal *Int*errupt

use super::{MmioDevice, PTE_FLAGS_FOR_DEVICE};
use crate::memmap::{
    GuestPhysicalAddress, HostPhysicalAddress, MemoryMap, page_table::constants::PAGE_SIZE,
};

use alloc::vec::Vec;
use fdt::{Fdt, standard_nodes::MemoryRegion};

#[allow(clippy::doc_markdown)]
/// CLINT: Core Local INTerrupt
/// Local interrupt controller
#[derive(Debug)]
pub struct Clint {
    /// Memory maps for memory mapped register.
    register_map_regions: Vec<MemoryRegion>,
}

impl MmioDevice for Clint {
    fn try_new(device_tree: &Fdt, compatibles: &[&str]) -> Option<Self> {
        let register_map_regions: Vec<MemoryRegion> = device_tree
            .find_compatible(compatibles)?
            .reg()
            .unwrap()
            .collect();

        Some(Clint {
            register_map_regions,
        })
    }

    fn memmap(&self) -> Vec<MemoryMap> {
        self.register_map_regions
            .clone()
            .into_iter()
            .map(|region| {
                let mut size = region.size.unwrap();
                let mut virt_start = GuestPhysicalAddress(region.starting_address as usize);
                let mut phys_start = HostPhysicalAddress(region.starting_address as usize);

                // change memory map region if region size is less than the page size.
                if phys_start % PAGE_SIZE != 0 && size < PAGE_SIZE {
                    size = PAGE_SIZE;
                    virt_start = virt_start - (virt_start % PAGE_SIZE);
                    phys_start = phys_start - (phys_start % PAGE_SIZE);
                }

                MemoryMap::new(
                    virt_start..virt_start + size,
                    phys_start..phys_start + size,
                    &PTE_FLAGS_FOR_DEVICE,
                )
            })
            .collect()
    }
}
