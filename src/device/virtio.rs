//! A virtualization standard for network and disk device drivers.

use super::{Device, DeviceEmulateError};
use crate::memmap::page_table::PteFlag;
use crate::memmap::{GuestPhysicalAddress, HostPhysicalAddress, MemoryMap};
use alloc::vec::Vec;
use core::slice::Iter;
use fdt::Fdt;

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
                    let region = node.reg().unwrap().next().unwrap();
                    let irq = node.property("interrupts").unwrap().value[0];
                    VirtIo {
                        base_addr: HostPhysicalAddress(region.starting_address as usize),
                        size: region.size.unwrap(),
                        irq,
                    }
                })
                .collect(),
        )
    }

    /// Emulate wrting to memory mapped register
    pub fn emulate_write(
        &mut self,
        dst_addr: HostPhysicalAddress,
        value: u32,
    ) -> Result<(), DeviceEmulateError> {
        let vio = self
            .0
            .iter()
            .find(|vio| vio.memmap().phys.contains(&dst_addr));

        match vio {
            Some(vio) => vio.emulate_write(dst_addr, value),
            None => Err(DeviceEmulateError::InvalidAddress),
        }
    }

    /// Return Virt IO list iterator
    pub fn iter(&self) -> Iter<'_, VirtIo> {
        self.0.iter()
    }
}

#[derive(Debug)]
pub struct VirtIo {
    base_addr: HostPhysicalAddress,
    size: usize,
    irq: u8,
}

impl VirtIo {
    pub fn irq(&self) -> u8 {
        self.irq
    }
}

impl Device for VirtIo {
    fn new(device_tree: &Fdt, node_path: &str) -> Self {
        let node = device_tree.find_all_nodes(node_path).next().unwrap();
        let region = node.reg().unwrap().next().unwrap();
        let irq = node.property("interrupts").unwrap().value[0];

        VirtIo {
            base_addr: HostPhysicalAddress(region.starting_address as usize),
            size: region.size.unwrap(),
            irq,
        }
    }

    fn size(&self) -> usize {
        self.size
    }

    fn paddr(&self) -> HostPhysicalAddress {
        self.base_addr
    }

    fn memmap(&self) -> MemoryMap {
        let vaddr = GuestPhysicalAddress(self.paddr().raw());
        MemoryMap::new(
            vaddr..vaddr + self.size(),
            self.paddr()..self.paddr() + self.size(),
            // deny write permission for emulation.
            &[PteFlag::Read, PteFlag::User, PteFlag::Valid],
        )
    }
}
