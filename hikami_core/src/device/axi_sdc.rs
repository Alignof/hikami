//! AXI SD Card
//!
//! Ref: [https://github.com/eugene-tarassov/vivado-risc-v/blob/master/patches/fpga-axi-sdc.c](https://github.com/eugene-tarassov/vivado-risc-v/blob/master/patches/fpga-axi-sdc.c)

use super::MmioDevice;
use crate::memmap::HostPhysicalAddress;

use alloc::string::{String, ToString};
use fdt::{Fdt, standard_nodes::MemoryRegion};

#[allow(clippy::doc_markdown)]
/// MMC: Multi Media Card
#[derive(Debug)]
pub struct Mmc {
    /// Device tree name
    name: String,
    /// Memory map for memory mapped register.
    #[allow(dead_code)]
    register_map_region: MemoryRegion,
}

impl Mmc {
    /// Return base address of memory mapped register region.
    #[allow(dead_code)]
    fn base_addr(&self) -> HostPhysicalAddress {
        HostPhysicalAddress(self.register_map_region.starting_address as usize)
    }
}

impl MmioDevice for Mmc {
    fn try_new(device_tree: &Fdt, compatibles: &[&str]) -> Option<Self> {
        let mmc_node = device_tree.find_compatible(compatibles)?;
        let register_map_region = mmc_node.reg().unwrap().next().unwrap();

        Some(Mmc {
            name: mmc_node.name.to_string(),
            register_map_region,
        })
    }

    fn name(&self) -> &str {
        &self.name
    }
}
