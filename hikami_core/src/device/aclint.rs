//! ACLINT: *A*dvanced *C*ore *L*ocal *Int*errupt

use super::{MmioDevice, PTE_FLAGS_FOR_DEVICE};
use crate::memmap::{
    GuestPhysicalAddress, HostPhysicalAddress, MemoryMap, page_table::constants::PAGE_SIZE,
};

use alloc::vec::Vec;
use fdt::{Fdt, standard_nodes::MemoryRegion};

/// MSWI: Machine level SoftWare Interrupt device
///
/// The MSWI device provides machine-level IPI functionality for a set of HARTs on a RISC-V platform.
/// It has an IPI register (MSIP) for each HART connected to the MSWI device.
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

/// MTIMER: Machine level TIMER device
///
/// The MTIMER device provides machine-level timer functionality for a set of HARTs on a RISC-V platform.
/// It has a single fixed-frequency monotonic time counter (MTIME) register and a time compare register (MTIMECMP) for each HART connected to the MTIMER device.
/// A MTIMER device not connected to any HART should only have a MTIME register and no MTIMECMP registers.
#[derive(Debug)]
pub struct Mtimer {
    /// Memory maps for memory mapped register.
    register_map_regions: Vec<MemoryRegion>,
}
impl MmioDevice for Mtimer {
    fn try_new(
        root_page_table_addr: HostPhysicalAddress,
        device_tree: &Fdt,
        compatibles: &[&str],
    ) -> Option<Self> {
        let clint_node = device_tree.find_compatible(compatibles)?;
        let register_map_regions: Vec<MemoryRegion> = clint_node.reg().unwrap().collect();

        Self::create_page_table(root_page_table_addr, &register_map_regions, clint_node.name);

        Some(Mtimer {
            register_map_regions,
        })
    }

    fn memmap(&self) -> Vec<MemoryMap> {
        self.register_map_regions
            .clone()
            .into_iter()
            .map(|region| {
                let mut size = region.size.unwrap();
                let mut virt_start = GuestPhysicalAddress(region.starting_address as usize);
                let mut phys_start = HostPhysicalAddress(region.starting_address as usize);

                // change memory map region if region size is less than the page size.
                if phys_start % PAGE_SIZE != 0 && size < PAGE_SIZE {
                    size = PAGE_SIZE;
                    virt_start = virt_start - (virt_start % PAGE_SIZE);
                    phys_start = phys_start - (phys_start % PAGE_SIZE);
                }

                MemoryMap::new(
                    virt_start..virt_start + size,
                    phys_start..phys_start + size,
                    &PTE_FLAGS_FOR_DEVICE,
                )
            })
            .collect()
    }
}

#[allow(clippy::doc_markdown)]
/// ACLINT: Advanced Core Local INTerrupt
/// Local interrupt controller
#[derive(Debug)]
pub struct Aclint {
    /// MSWI
    mswi: Mswi,
    /// MTIMER
    mtimer: Mtimer,
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
            mswi: Mswi::try_new(root_page_table_addr, device_tree, mswi_compatibles)?,
            mtimer: Mtimer::try_new(root_page_table_addr, device_tree, mtimer_compatibles)?,
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
