//! RTC: Real Time Clock.

use super::MmioDevice;
use crate::memmap::MemoryMap;

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
    fn try_new(device_tree: &Fdt, compatibles: &[&str]) -> Option<Self> {
        let register_map_regions: Vec<MemoryRegion> = device_tree
            .find_compatible(compatibles)?
            .reg()
            .unwrap()
            .collect();

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
