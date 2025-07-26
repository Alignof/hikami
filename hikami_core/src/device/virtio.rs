//! A virtualization standard for network and disk device drivers.

use super::MmioDevice;
use crate::memmap::MemoryMap;

use alloc::vec::Vec;
use core::slice::Iter;
use fdt::{Fdt, node::FdtNode, standard_nodes::MemoryRegion};

/// A virtualization standard for network and disk device drivers.
/// Since more than one may be found, we will temporarily use the first one.
#[derive(Debug)]
pub struct VirtIoList(Vec<VirtIo>);

impl VirtIoList {
    /// Create each Virt IO data when device has multiple IOs.
    pub fn new(device_tree: &Fdt, node_path: &str) -> Self {
        VirtIoList(
            device_tree
                .find_all_nodes(node_path)
                .map(|node| {
                    let register_map_regions: Vec<MemoryRegion> = node.reg().unwrap().collect();
                    let irq = node.property("interrupts").unwrap().value[0];
                    VirtIo {
                        register_map_regions,
                        irq,
                    }
                })
                .collect(),
        )
    }

    /// Return Virt IO list iterator
    pub fn iter(&self) -> Iter<'_, VirtIo> {
        self.0.iter()
    }
}

/// Virtualization standard for IO device.
#[derive(Debug)]
pub struct VirtIo {
    /// Memory maps for memory mapped register.
    register_map_regions: Vec<MemoryRegion>,
    /// Interrupt Reqeust bit.
    irq: u8,
}

impl VirtIo {
    /// Create self with fdt node
    pub fn new_with_node(root_page_table_addr: HostPhysicalAddress, virtio_node: &FdtNode) -> Self {
        let register_map_regions: Vec<MemoryRegion> = virtio_node.reg().unwrap().collect();

        Self::create_page_table(
            root_page_table_addr,
            &register_map_regions,
            virtio_node.name,
        );

        VirtIo {
            register_map_regions,
            irq: virtio_node.property("interrupts").unwrap().value[0],
        }
    }

    /// Return `irq`.
    pub fn irq(&self) -> u8 {
        self.irq
    }
}

impl MmioDevice for VirtIo {
    fn try_new(
        _root_page_table_addr: HostPhysicalAddress,
        _device_tree: &Fdt,
        _compatibles: &[&str],
    ) -> Option<Self> {
        unreachable!("use `VirtIo::new_with_node` instead.")
    }

    fn memmap(&self) -> Vec<MemoryMap> {
        self.register_map_regions
            .clone()
            .into_iter()
            .map(MemoryMap::from)
            .collect()
    }
}
