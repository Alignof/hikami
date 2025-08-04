//! MSWI: Machine level SoftWare Interrupt device
//!
//! The MSWI device provides machine-level IPI functionality for a set of HARTs on a RISC-V platform.
//! It has an IPI register (MSIP) for each HART connected to the MSWI device.

use super::super::{DeviceEmulateError, MmioDevice};
use crate::memmap::HostPhysicalAddress;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use fdt::{Fdt, standard_nodes::MemoryRegion};

/// MSWI Register size (4 Bytes)
const MSWI_REG_SIZE: usize = 0x4;

/// MSWI: Machine level SoftWare Interrupt device
#[derive(Debug)]
pub struct Mswi {
    /// Device tree name
    name: String,
    /// Memory maps for memory mapped register.
    register_map_regions: Vec<MemoryRegion>,
}

impl Mswi {
    /// Return mswi device base address.
    fn base_addr(&self) -> HostPhysicalAddress {
        HostPhysicalAddress(self.register_map_regions[0].starting_address as usize)
    }

    /// Return mswi device register size
    fn size(&self) -> usize {
        self.register_map_regions[0]
            .size
            .expect("Mswi does not have size")
    }

    /// Emulate reading mswi register.
    ///
    /// # Errors
    /// It will return an error if `dst_addr` is out of range.
    pub fn emulate_loading(
        &self,
        hart_id: usize,
        guest_hart_id: usize,
        dst_addr: HostPhysicalAddress,
    ) -> Result<u32, DeviceEmulateError> {
        if !(self.base_addr()..self.base_addr() + self.size()).contains(&dst_addr) {
            return Err(DeviceEmulateError::InvalidAddress);
        }

        let offset = dst_addr.raw() - self.base_addr().raw();
        let calced_guest_hart_id = offset / MSWI_REG_SIZE;

        assert_eq!(guest_hart_id, calced_guest_hart_id);

        let phys_hart_ptr = (self.base_addr().raw() + hart_id * MSWI_REG_SIZE) as *mut u32;

        unsafe { Ok(phys_hart_ptr.read_volatile()) }
    }

    /// Emulate storing mswi register.
    ///
    /// # Errors
    /// It will return an error if `dst_addr` is out of range.
    pub fn emulate_storing(
        &mut self,
        hart_id: usize,
        guest_hart_id: usize,
        dst_addr: HostPhysicalAddress,
        value: u32,
    ) -> Result<(), DeviceEmulateError> {
        if !(self.base_addr()..self.base_addr() + self.size()).contains(&dst_addr) {
            return Err(DeviceEmulateError::InvalidAddress);
        }

        let offset = dst_addr.raw() - self.base_addr().raw();
        let calced_guest_hart_id = offset / MSWI_REG_SIZE;

        assert_eq!(guest_hart_id, calced_guest_hart_id);

        let phys_hart_ptr = (self.base_addr().raw() + hart_id * MSWI_REG_SIZE) as *mut u32;

        unsafe {
            phys_hart_ptr.write_volatile(value);
        }

        Ok(())
    }
}

impl MmioDevice for Mswi {
    fn try_new(
        _root_page_table_addr: HostPhysicalAddress,
        device_tree: &Fdt,
        compatibles: &[&str],
    ) -> Option<Self> {
        let clint_node = device_tree.find_compatible(compatibles)?;
        let register_map_regions: Vec<MemoryRegion> = clint_node.reg().unwrap().collect();

        Some(Mswi {
            name: clint_node.name.to_string(),
            register_map_regions,
        })
    }

    fn name(&self) -> &str {
        &self.name
    }
}
