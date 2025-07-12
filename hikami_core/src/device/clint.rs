//! CLINT: *C*ore *L*ocal *Int*errupt

use super::MmioDevice;
use crate::memmap::MemoryMap;

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
            .map(|region| MemoryMap::from(region))
            .collect()
    }
}
