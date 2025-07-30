//! MSWI: Machine level SoftWare Interrupt device
//!
//! The MSWI device provides machine-level IPI functionality for a set of HARTs on a RISC-V platform.
//! It has an IPI register (MSIP) for each HART connected to the MSWI device.

use super::MmioDevice;
use crate::memmap::{HostPhysicalAddress, MemoryMap};

use alloc::vec::Vec;
use fdt::{Fdt, standard_nodes::MemoryRegion};

/// MSWI: Machine level SoftWare Interrupt device
#[derive(Debug)]
pub struct Mswi {
    /// Memory maps for memory mapped register.
    register_map_regions: Vec<MemoryRegion>,
}
impl MmioDevice for Mswi {
    fn try_new(
        root_page_table_addr: HostPhysicalAddress,
        device_tree: &Fdt,
        compatibles: &[&str],
    ) -> Option<Self> {
        let clint_node = device_tree.find_compatible(compatibles)?;
        let register_map_regions: Vec<MemoryRegion> = clint_node.reg().unwrap().collect();

        Self::create_page_table(root_page_table_addr, &register_map_regions, clint_node.name);

        Some(Mswi {
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
