//! SDHCI: SD Host Controller Interface

use super::MmioDevice;
use crate::memmap::{HostPhysicalAddress, MemoryMap};

use alloc::vec::Vec;
use fdt::{Fdt, standard_nodes::MemoryRegion};

/// SDHCI: SD Host Controller Interface
#[derive(Debug)]
pub struct Mmc {
    /// Memory maps for memory mapped register.
    register_map_regions: Vec<MemoryRegion>,
}

impl MmioDevice for Mmc {
    fn try_new(
        root_page_table_addr: HostPhysicalAddress,
        device_tree: &Fdt,
        compatibles: &[&str],
    ) -> Option<Self> {
        let mmc_node = device_tree.find_compatible(compatibles)?;
        let register_map_regions: Vec<MemoryRegion> = mmc_node.reg().unwrap().collect();

        Self::create_page_table(root_page_table_addr, &register_map_regions, mmc_node.name);

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
