//! hikami library

#![no_std]
// TODO: FIX AND REMOVE IT!!!
#![allow(static_mut_refs)]

extern crate alloc;
pub mod device;
pub mod emulate_extension;
pub mod guest;
pub mod h_extension;
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
use memmap::page_table::{PteFlag, sv39x4::FIRST_LV_PAGE_TABLE_LEN};
use memmap::{
    GuestPhysicalAddress, HostPhysicalAddress, MemoryMap, page_table, page_table::PageTableEntry,
};

/// Singleton for this hypervisor.
pub static mut HYPERVISOR_DATA: Mutex<OnceCell<HypervisorData>> = Mutex::new(OnceCell::new());

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
}

impl HypervisorData {
    /// Initialize hypervisor.
    ///
    /// # Panics
    /// It will be panic when parsing device tree failed.
    #[must_use]
    pub fn new(root_page_table_addr: HostPhysicalAddress, device_tree: Fdt) -> Self {
        // init page table
        page_table::sv39x4::initialize_page_table(root_page_table_addr);

        Self::map_all_device_region(root_page_table_addr, device_tree);

        HypervisorData {
            // fix to zero for now
            //
            // TODO: save `hart_id` and `guest_hart_id` to `ContextData` and decide current guest
            // hart id.
            current_guest_hart: 0,
            guests: [const { None }; MAX_HART_NUM],
            devices: Devices::new(root_page_table_addr, device_tree),
        }
    }

    /// Map all regions for memory mapped divices.
    fn map_all_device_region(root_page_table_addr: HostPhysicalAddress, device_tree: Fdt) {
        use PteFlag::{Accessed, Dirty, Exec, Read, User, Valid, Write};
        const G_STAGE_PTE_FLAGS: &[PteFlag; 7] = &[Dirty, Accessed, Read, Write, Exec, User, Valid];

        let dram_start = device_tree
            .find_node("/memory")
            .unwrap()
            .reg()
            .unwrap()
            .next()
            .expect("couldn't get memory region")
            .starting_address as usize;

        crate::println!(
            "map memory mapped device region Mapping {:#x} bytes to GPA [{:#x} - {:#x}]",
            dram_start,
            0,
            dram_start
        );

        // TODO: avoid hard coding the address.
        // 0x0 .. 0x8000_0000
        // 0x8000_0000 .. 0x9000_0000
        // 0xb000_0000 .. 0x4_0000_0000
        let all_memory_map = [
            MemoryMap::new(
                GuestPhysicalAddress(0x0)..GuestPhysicalAddress(dram_start),
                HostPhysicalAddress(0x0)..HostPhysicalAddress(dram_start),
                G_STAGE_PTE_FLAGS,
            ),
            MemoryMap::new(
                GuestPhysicalAddress(0x8008_0000)..GuestPhysicalAddress(0x9000_0000),
                HostPhysicalAddress(0x8008_0000)..HostPhysicalAddress(0x9000_0000),
                G_STAGE_PTE_FLAGS,
            ),
            MemoryMap::new(
                GuestPhysicalAddress(0x1_8000_0000)..GuestPhysicalAddress(0x4_8000_0000),
                HostPhysicalAddress(0x1_8000_0000)..HostPhysicalAddress(0x4_8000_0000),
                G_STAGE_PTE_FLAGS,
            ),
            MemoryMap::new(
                GuestPhysicalAddress(0x80_0000_0000)..GuestPhysicalAddress(0x100_0000_0000),
                HostPhysicalAddress(0x80_0000_0000)..HostPhysicalAddress(0x100_0000_0000),
                G_STAGE_PTE_FLAGS,
            ),
        ];
        page_table::sv39x4::generate_page_table(root_page_table_addr, &all_memory_map);
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

    /// Return current hart's guest as mutable.
    ///
    /// # Panics
    /// It will be panic if current HART's guest data is empty.
    #[must_use]
    pub fn guest_mut(&mut self) -> &mut Guest {
        self.guests[self.current_guest_hart]
            .as_mut()
            .expect("guest data not found")
    }

    /// Create and register new guest.
    ///
    /// # Panics
    /// It will be panic if `hart_id` is greater than `MAX_HART_NUM`.
    pub fn register_new_guest(
        &mut self,
        hart_id: usize,
        root_page_table: &'static [PageTableEntry; FIRST_LV_PAGE_TABLE_LEN],
        guest_kernel: &'static [u8],
        guest_dtb: &'static [u8],
        guest_initrd: &'static [u8],
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
            root_page_table,
            guest_dtb,
            guest_initrd,
        );

        // load guest elf `from GUEST_KERNEL`
        let guest_elf = unsafe {
            ElfBytes::<AnyEndian>::minimal_parse(core::slice::from_raw_parts(
                guest_kernel.as_ptr(),
                guest_kernel.len(),
            ))
            .unwrap()
        };

        // load guest image
        let guest_entry_point =
            unsafe { new_guest.load_guest_elf(&guest_elf, guest_kernel.as_ptr()) };

        self.guests[guest_hart_id] = Some(new_guest);

        (guest_hart_id, guest_entry_point)
    }
}

unsafe extern "C" {
    /// stack top (defined in `memory.x`)
    pub static _stack_start: u8;
    /// start of heap (defined in `memory.x`)
    pub static mut _start_heap: u8;
    /// heap size (defined in `memory.x`)
    pub static _hv_heap_size: u8;
    /// boot stack top (defined in `memory.x`)
    pub static _top_b_stack: u8;
    /// start of bss and sbss section.
    pub static _start_bss: u8;
    /// end of bss and sbss section.
    pub static _end_bss: u8;
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
