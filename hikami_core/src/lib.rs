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

use device::Devices;
use guest::Guest;
use memmap::HostPhysicalAddress;
use memmap::constant::MAX_HART_NUM;

use fdt::Fdt;
use spin::Mutex;

/// Singleton for this hypervisor.
pub static mut HYPERVISOR_DATA: Mutex<OnceCell<HypervisorData>> = Mutex::new(OnceCell::new());

/// Global data for hypervisor.
///
/// FIXME: Rename me!
#[derive(Debug)]
pub struct HypervisorData {
    /// Current hart id (zero indexed).
    current_hart: usize,
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
    pub fn new(device_tree: Fdt) -> Self {
        HypervisorData {
            current_hart: 0,
            guests: [const { None }; MAX_HART_NUM],
            devices: Devices::new(device_tree),
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
        self.guests[self.current_hart]
            .as_ref()
            .expect("guest data not found")
    }

    /// Add new guest data.
    ///
    /// # Panics
    /// It will be panic if `hart_id` is greater than `MAX_HART_NUM`.
    pub fn register_guest(&mut self, new_guest: Guest) {
        let hart_id = new_guest.hart_id();
        assert!(hart_id < MAX_HART_NUM);
        self.guests[hart_id] = Some(new_guest);
    }
}

/// Guest kernel image
#[unsafe(link_section = ".guest_kernel")]
pub static GUEST_KERNEL: [u8; include_bytes!("../../guest_image/vmlinux").len()] =
    *include_bytes!("../../guest_image/vmlinux");

/// Device tree blob that is passed to guest
#[unsafe(link_section = ".guest_dtb")]
pub static GUEST_DTB: [u8; include_bytes!("../../guest_image/guest.dtb").len()] =
    *include_bytes!("../../guest_image/guest.dtb");

/// Guest intird
#[unsafe(link_section = ".guest_initrd")]
pub static GUEST_INITRD: [u8; include_bytes!("../../guest_image/initrd").len()] =
    *include_bytes!("../../guest_image/initrd");

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
