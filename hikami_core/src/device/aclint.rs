//! ACLINT: *A*dvanced *C*ore *L*ocal *Int*errupt

mod mswi;
mod mtimer;

use super::{MmioDevice, PTE_FLAGS_FOR_DEVICE};
use crate::memmap::{HostPhysicalAddress, MemoryMap};

use alloc::vec::Vec;
use fdt::{Fdt, standard_nodes::MemoryRegion};

#[allow(clippy::doc_markdown)]
/// ACLINT: Advanced Core Local INTerrupt
/// Local interrupt controller
#[derive(Debug)]
pub struct Aclint {
    /// MSWI
    mswi: mswi::Mswi,
    /// MTIMER
    mtimer: mtimer::Mtimer,
}

impl Aclint {
    pub fn try_new_aclint(
        root_page_table_addr: HostPhysicalAddress,
        device_tree: &Fdt,
        mswi_compatibles: &[&str],
        mtimer_compatibles: &[&str],
    ) -> Option<Self> {
        let mswi_node = device_tree.find_compatible(mswi_compatibles)?;
        let mtimer_node = device_tree.find_compatible(mtimer_compatibles)?;
        let mut register_map_regions: Vec<MemoryRegion> = mswi_node.reg().unwrap().collect();
        register_map_regions.append(&mut mtimer_node.reg().unwrap().collect());

        Self::create_page_table(
            root_page_table_addr,
            &register_map_regions,
            mtimer_node.name,
        );

        Some(Aclint {
            mswi: mswi::Mswi::try_new(root_page_table_addr, device_tree, mswi_compatibles)?,
            mtimer: mtimer::Mtimer::try_new(root_page_table_addr, device_tree, mtimer_compatibles)?,
        })
    }
}

impl MmioDevice for Aclint {
    fn try_new(
        _root_page_table_addr: HostPhysicalAddress,
        _device_tree: &Fdt,
        _compatibles: &[&str],
    ) -> Option<Self> {
        unreachable!("Use `Aclint::try_new_aclint` instead");
    }

    fn memmap(&self) -> Vec<MemoryMap> {
        let mut memory_maps = self.mswi.memmap();
        memory_maps.append(&mut self.mtimer.memmap());

        memory_maps
    }
}
