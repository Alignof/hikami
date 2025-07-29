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
use memmap::page_table::sv39x4::FIRST_LV_PAGE_TABLE_LEN;
use memmap::{GuestPhysicalAddress, HostPhysicalAddress, page_table::PageTableEntry};

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
            &root_page_table,
            &guest_dtb,
            &guest_initrd,
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
