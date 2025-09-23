//! MTIMER: Machine level TIMER device
//!
//! The MTIMER device provides machine-level timer functionality for a set of HARTs on a RISC-V platform.
//! It has a single fixed-frequency monotonic time counter (MTIME) register and a time compare register (MTIMECMP) for each HART connected to the MTIMER device.
//! A MTIMER device not connected to any HART should only have a MTIME register and no MTIMECMP registers.

use super::MmioDevice;
use crate::memmap::HostPhysicalAddress;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use fdt::{Fdt, standard_nodes::MemoryRegion};

/// MTIMER: Machine level TIMER device
#[derive(Debug)]
pub struct Mtimer {
    /// Device tree name
    name: String,

    #[allow(dead_code)]
    /// Memory maps for memory mapped register.
    register_map_regions: Vec<MemoryRegion>,
}
impl MmioDevice for Mtimer {
    fn try_new(
        _root_page_table_addr: HostPhysicalAddress,
        device_tree: &Fdt,
        compatibles: &[&str],
    ) -> Option<Self> {
        let clint_node = device_tree.find_compatible(compatibles)?;
        let register_map_regions: Vec<MemoryRegion> = clint_node.reg().unwrap().collect();

        Some(Mtimer {
            name: clint_node.name.to_string(),
            register_map_regions,
        })
    }

    fn name(&self) -> &str {
        &self.name
    }
}
