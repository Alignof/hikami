//! RTC: Real Time Clock.

use super::MmioDevice;
use crate::memmap::{HostPhysicalAddress, MemoryMap};

use alloc::vec::Vec;
use fdt::{Fdt, standard_nodes::MemoryRegion};

/// RTC: Real Time Clock.
/// An electronic device that measures the passage of time.
#[derive(Debug)]
pub struct Rtc {
    /// Memory maps for memory mapped register.
    register_map_regions: Vec<MemoryRegion>,
}

impl MmioDevice for Rtc {
    fn try_new(
        root_page_table_addr: HostPhysicalAddress,
        device_tree: &Fdt,
        compatibles: &[&str],
    ) -> Option<Self> {
        let rtc_node = device_tree.find_compatible(compatibles)?;
        let register_map_regions: Vec<MemoryRegion> = rtc_node.reg().unwrap().collect();

        Self::create_page_table(root_page_table_addr, &register_map_regions, rtc_node.name);

        Some(Rtc {
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
