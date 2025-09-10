//! CLINT: *C*ore *L*ocal *Int*errupt

use super::MmioDevice;
use crate::memmap::HostPhysicalAddress;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use fdt::{Fdt, standard_nodes::MemoryRegion};

#[allow(clippy::doc_markdown)]
/// CLINT: Core Local INTerrupt
/// Local interrupt controller
#[derive(Debug)]
pub struct Clint {
    /// Device tree name
    name: String,
    /// Memory maps for memory mapped register.
    register_map_regions: Vec<MemoryRegion>,
}

impl MmioDevice for Clint {
    fn try_new(
        root_page_table_addr: HostPhysicalAddress,
        device_tree: &Fdt,
        compatibles: &[&str],
    ) -> Option<Self> {
        let clint_node = device_tree.find_compatible(compatibles)?;
        let register_map_regions: Vec<MemoryRegion> = clint_node.reg().unwrap().collect();

        Some(Clint {
            name: clint_node.name.to_string(),
            register_map_regions,
        })
    }

    fn name(&self) -> &str {
        &self.name
    }
}
