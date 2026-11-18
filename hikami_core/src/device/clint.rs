//! CLINT: *C*ore *L*ocal *Int*errupt

use super::MmioDevice;
use crate::device::DeviceEmulateError;
use crate::memmap::{GuestPhysicalAddress, HostPhysicalAddress, page_table};

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use fdt::{Fdt, standard_nodes::MemoryRegion};

/// Clint Register size (8 Bytes)
const CLINT_REG_SIZE: usize = 0x8;

/// Base address of MTIMECMP
const MTIMECMP_BASE_OFFSET: usize = 0x4000;

/// Address of MTIME
const MTIME_OFFSET: usize = 0xbff8;

#[allow(clippy::doc_markdown)]
/// CLINT: Core Local INTerrupt
/// Local interrupt controller
#[derive(Debug)]
pub struct Clint {
    /// Device tree name
    name: String,

    #[allow(dead_code)]
    /// Memory maps for memory mapped register.
    register_map_regions: Vec<MemoryRegion>,
}

impl Clint {
    /// Return mtimer device base address.
    fn base_addr(&self) -> HostPhysicalAddress {
        HostPhysicalAddress(self.register_map_regions[0].starting_address as usize)
    }

    /// Return mtimer device register size
    fn size(&self) -> usize {
        self.register_map_regions[0]
            .size
            .expect("mtimer does not have size")
    }

    /// Emulate reading mtimer register.
    ///
    /// # Errors
    /// It will return an error if `dst_addr` is out of range.
    pub fn emulate_loading(
        &self,
        hart_id: usize,
        guest_hart_id: usize,
        dst_addr: HostPhysicalAddress,
    ) -> Result<u32, DeviceEmulateError> {
        if !(self.base_addr() + MTIMECMP_BASE_OFFSET..self.base_addr() + self.size())
            .contains(&dst_addr)
        {
            return Err(DeviceEmulateError::InvalidAddress);
        }

        let mtimecmp_offset = dst_addr.raw() - (self.base_addr().raw() + MTIMECMP_BASE_OFFSET);

        if mtimecmp_offset == MTIME_OFFSET {
            let phys_hart_ptr = dst_addr.raw() as *mut u32;
            return unsafe { Ok(phys_hart_ptr.read_volatile()) };
        }

        let calced_guest_hart_id = mtimecmp_offset / CLINT_REG_SIZE;

        assert_eq!(guest_hart_id, calced_guest_hart_id);

        let phys_hart_ptr = (self.base_addr().raw() + hart_id * CLINT_REG_SIZE) as *mut u32;

        unsafe { Ok(phys_hart_ptr.read_volatile()) }
    }

    /// Emulate storing mtimer register.
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
        if !(self.base_addr() + MTIMECMP_BASE_OFFSET..self.base_addr() + self.size())
            .contains(&dst_addr)
        {
            return Err(DeviceEmulateError::InvalidAddress);
        }

        let mtimecmp_offset = dst_addr.raw() - (self.base_addr().raw() + MTIMECMP_BASE_OFFSET);

        if mtimecmp_offset == MTIME_OFFSET {
            unsafe {
                let phys_hart_ptr = dst_addr.raw() as *mut u32;
                phys_hart_ptr.write_volatile(value);
            }
            return Ok(());
        }

        let calced_guest_hart_id = mtimecmp_offset / CLINT_REG_SIZE;

        assert_eq!(guest_hart_id, calced_guest_hart_id);

        let phys_hart_ptr = (self.base_addr().raw() + hart_id * CLINT_REG_SIZE) as *mut u32;

        unsafe {
            phys_hart_ptr.write_volatile(value);
        }

        Ok(())
    }
}

impl MmioDevice for Clint {
    fn try_new(
        _root_page_table_addr: HostPhysicalAddress,
        device_tree: &Fdt,
        compatibles: &[&str],
    ) -> Option<Self> {
        let clint_node = device_tree.find_compatible(compatibles)?;
        let register_map_regions: Vec<MemoryRegion> = clint_node.reg().unwrap().collect();

        Self::invalidate_page_table(&register_map_regions, clint_node.name);

        Some(Clint {
            name: clint_node.name.to_string(),
            register_map_regions,
        })
    }

    /// Invalidate page table
    fn invalidate_page_table(memory_regions: &[MemoryRegion], node_name: &str) {
        assert!(memory_regions.len() == 1);
        let memmap = memory_regions[0];
        // invalidate memory map to emulate plic registers.
        let invalidate_range = GuestPhysicalAddress(memmap.starting_address as usize)
            ..GuestPhysicalAddress(memmap.starting_address as usize + memmap.size.unwrap());

        crate::println!(
            "[Device Unmap] {}: {:#x}..{:#x}",
            node_name,
            invalidate_range.start.raw() + MTIMECMP_BASE_OFFSET,
            invalidate_range.end.raw()
        );

        page_table::sv39x4::invalidate_address_range(invalidate_range)
            .expect("failed to invalidate mtimer registers range");
    }

    fn name(&self) -> &str {
        &self.name
    }
}
