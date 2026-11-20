//! hikami library

#![no_std]

#[cfg(all(feature = "identity_map", feature = "iommu"))]
compile_error!("features `identity_map` and `iommu` are mutually exclusive");
#[cfg(not(any(feature = "identity_map", feature = "iommu")))]
compile_error!("either `identity_map` or `iommu` feature must be enabled");

extern crate alloc;
pub mod board;
pub mod device;
pub mod emulate_extension;
pub mod guest;
pub mod h_extension;
pub mod isa;
pub mod log;
pub mod memmap;
pub mod trap;

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::cell::OnceCell;

use elf::{ElfBytes, endian::AnyEndian};
use fdt::Fdt;
use spin::Mutex;

use device::Devices;
use guest::Guest;
use memmap::constant::MAX_HART_NUM;
use memmap::page_table::PteFlag;
use memmap::{GuestPhysicalAddress, HostPhysicalAddress, MemoryMap, page_table};

/// Singleton for this hypervisor.
pub static HYPERVISOR_DATA: Mutex<OnceCell<HypervisorData>> = Mutex::new(OnceCell::new());

/// Global data for hypervisor.
///
/// FIXME: Rename me!
#[derive(Debug)]
pub struct HypervisorData {
    /// Current guest hart id (zero indexed).
    current_guest_hart: usize,
    /// Guests data
    guests: [Option<guest::Guest>; MAX_HART_NUM],
    /// Devices data.
    devices: device::Devices,
    /// Host physical regions that must not be mapped to guests.
    ///
    /// The hypervisor itself and `/reserved-memory` of the host device tree.
    reserved_regions: Vec<core::ops::Range<usize>>,
    /// Host memory that backs guest memory.
    #[cfg(feature = "iommu")]
    host_memory_pool: memmap::host_memory_pool::HostMemoryPool,
}

// # Safety
// `HypervisorData` is only accessed through `HYPERVISOR_DATA` lock.
// Raw pointers in it (e.g. `fdt::standard_nodes::MemoryRegion`) hold physical addresses of
// devices, which are the same on all harts.
unsafe impl Send for HypervisorData {}

impl HypervisorData {
    /// Initialize hypervisor.
    ///
    /// # Panics
    /// It will be panic when parsing device tree failed.
    #[must_use]
    #[cfg_attr(not(feature = "iommu"), allow(unused_variables))]
    pub fn new(
        device_tree: Fdt,
        host_dtb_addr: HostPhysicalAddress,
        hypervisor_region: core::ops::Range<HostPhysicalAddress>,
    ) -> Self {
        // init page table
        page_table::sv39x4::initialize_page_table();

        Self::map_all_device_region(&device_tree);

        let mut reserved_regions = memmap::reserved_memory_regions(&device_tree);
        reserved_regions.push(hypervisor_region.start.raw()..hypervisor_region.end.raw());
        for region in &reserved_regions {
            crate::println!("[Reserved] {:#x}..{:#x}", region.start, region.end);
        }

        #[cfg(feature = "iommu")]
        let host_memory_pool = {
            let host_dtb_region =
                host_dtb_addr.raw()..host_dtb_addr.raw() + device_tree.total_size();
            let pool = memmap::host_memory_pool::HostMemoryPool::new(
                &memmap::memory_regions(&device_tree),
                &[reserved_regions.as_slice(), &[host_dtb_region]].concat(),
            );
            for range in pool.free_ranges() {
                crate::println!("[Guest Memory Pool] {:#x}..{:#x}", range.start, range.end);
            }
            pool
        };

        HypervisorData {
            // fix to zero for now
            //
            // TODO: save `hart_id` and `guest_hart_id` to `ContextData` and decide current guest
            // hart id.
            current_guest_hart: 0,
            guests: [const { None }; MAX_HART_NUM],
            devices: Devices::new(device_tree),
            reserved_regions,
            #[cfg(feature = "iommu")]
            host_memory_pool,
        }
    }

    /// Map all regions except the host DRAM as devices.
    ///
    /// Regions emulated by the hypervisor are unmapped later by each device.
    fn map_all_device_region(device_tree: &Fdt) {
        use PteFlag::{Accessed, Dirty, Exec, Read, User, Valid, Write};
        const G_STAGE_PTE_FLAGS: &[PteFlag; 7] = &[Dirty, Accessed, Read, Write, Exec, User, Valid];
        /// End of guest physical address space in Sv39x4 (41 bit).
        const GPA_SPACE_END: usize = 1 << 41;

        let host_dram = memmap::memory_regions(device_tree);
        let device_maps: Vec<MemoryMap> = memmap::subtract_ranges(&(0..GPA_SPACE_END), &host_dram)
            .into_iter()
            .map(|region| {
                crate::println!("[Device Map] {:#x}..{:#x}", region.start, region.end);
                MemoryMap::new(
                    GuestPhysicalAddress(region.start)..GuestPhysicalAddress(region.end),
                    HostPhysicalAddress(region.start)..HostPhysicalAddress(region.end),
                    G_STAGE_PTE_FLAGS,
                )
            })
            .collect();
        page_table::sv39x4::generate_page_table(&device_maps);
    }

    /// Return Device objects.
    ///
    /// # Panics
    /// It will be panic if devices are uninitialized.
    #[must_use]
    pub fn devices(&mut self) -> &mut device::Devices {
        &mut self.devices
    }

    /// Return current hart's guest.
    ///
    /// # Panics
    /// It will be panic if current HART's guest data is empty.
    #[must_use]
    pub fn guest(&self) -> &Guest {
        self.guests[self.current_guest_hart]
            .as_ref()
            .expect("guest data not found")
    }

    /// Create and register new guest.
    ///
    /// # Panics
    /// It will be panic if `hart_id` is greater than `MAX_HART_NUM`.
    pub fn register_new_guest(
        &mut self,
        hart_id: usize,
        stack_top_addr: HostPhysicalAddress,
        board: &board::Board,
    ) -> (usize, GuestPhysicalAddress) {
        // decide guest HART ID
        let guest_hart_id = self
            .guests
            .iter()
            .position(|x| x.is_none())
            .expect("guests are full");

        // create new guest data
        let new_guest = Guest::new(
            hart_id,
            guest_hart_id,
            stack_top_addr,
            board,
            &self.reserved_regions,
            #[cfg(feature = "iommu")]
            &mut self.host_memory_pool,
        );

        // load guest elf `from GUEST_KERNEL`
        let guest_elf = unsafe {
            ElfBytes::<AnyEndian>::minimal_parse(core::slice::from_raw_parts(
                board.guest_kernel.as_ptr(),
                board.guest_kernel.len(),
            ))
            .unwrap()
        };

        // load guest image
        let guest_entry_point = unsafe {
            new_guest.load_guest_elf(
                &guest_elf,
                board.guest_kernel.as_ptr(),
                board.guest_kernel_base,
            )
        };

        self.guests[guest_hart_id] = Some(new_guest);

        (guest_hart_id, guest_entry_point)
    }
}

/// Aligned page size memory block
#[repr(C, align(0x1000))]
pub struct PageBlock([u8; 0x1000]);

impl PageBlock {
    /// Return aligned address of page size memory block.
    #[must_use]
    pub fn alloc() -> HostPhysicalAddress {
        let mut host_physical_block_as_vec: Vec<core::mem::MaybeUninit<PageBlock>> =
            Vec::with_capacity(1);
        unsafe {
            host_physical_block_as_vec.set_len(1);
        }

        let host_physical_block_slice = host_physical_block_as_vec.into_boxed_slice();
        HostPhysicalAddress(Box::into_raw(host_physical_block_slice) as *const u8 as usize)
    }
}
