//! SDHCI: SD Host Controller Interface

use super::MmioDevice;
use crate::memmap::MemoryMap;

use alloc::vec::Vec;
use fdt::{Fdt, standard_nodes::MemoryRegion};

/// SDHCI: SD Host Controller Interface
#[derive(Debug)]
pub struct Mmc {
    /// Memory maps for memory mapped register.
    register_map_regions: Vec<MemoryRegion>,
}

impl MmioDevice for Mmc {
    fn try_new(device_tree: &Fdt, compatibles: &[&str]) -> Option<Self> {
        let register_map_regions: Vec<MemoryRegion> = device_tree
            .find_compatible(compatibles)?
            .reg()
            .unwrap()
            .collect();

        Some(Mmc {
            register_map_regions,
        })
    }

    fn memmap(&self) -> Vec<MemoryMap> {
        self.register_map_regions
            .clone()
            .into_iter()
            .map(MemoryMap::from)
            .collect()
    }
}
