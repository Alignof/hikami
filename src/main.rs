#![no_main]
#![no_std]

extern crate alloc;
mod device;
mod guest;
mod h_extension;
mod hypervisor_init;
mod machine_init;
mod memmap;
mod sbi;
mod trap;

use core::arch::asm;
use core::cell::OnceCell;
use core::panic::PanicInfo;

use fdt::Fdt;
use linked_list_allocator::LockedHeap;
use riscv_rt::entry;
use spin::Mutex;

use crate::device::Devices;
use crate::guest::Guest;
use crate::machine_init::mstart;
use crate::memmap::constant::{DRAM_BASE, MAX_HART_NUM, STACK_SIZE_PER_HART};
use crate::sbi::Sbi;

#[global_allocator]
// static mut ALLOCATOR: WildScreenAlloc = WildScreenAlloc::empty();
/// Global allocator.
static ALLOCATOR: LockedHeap = LockedHeap::empty();

/// Singleton for this hypervisor.
///
/// TODO: change to `Mutex<OnceCell<HypervisorData>>`?
static mut HYPERVISOR_DATA: Mutex<OnceCell<HypervisorData>> = Mutex::new(OnceCell::new());

/// Singleton for SBI handler.
static SBI: Mutex<OnceCell<Sbi>> = Mutex::new(OnceCell::new());

/// Device tree blob that is passed to guest
#[link_section = ".guest_dtb"]
static GUEST_DTB: [u8; include_bytes!("../guest.dtb").len()] = *include_bytes!("../guest.dtb");

extern "C" {
    /// stack top (defined in `memory.x`)
    static _stack_start: u8;
    /// start of heap (defined in `memory.x`)
    static mut _start_heap: u8;
    /// heap size (defined in `memory.x`)
    static _hv_heap_size: u8;
    /// machine stack top (defined in `memory.x`)
    static _top_m_stack: u8;
}

/// Panic handler
#[panic_handler]
pub fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {
        riscv::asm::wfi();
    }
}

/// Global data for hypervisor.
///
/// FIXME: Rename me!
#[derive(Debug)]
pub struct HypervisorData {
    current_hart: usize,
    guests: [Option<guest::Guest>; MAX_HART_NUM],
    devices: device::Devices,
}

impl HypervisorData {
    /// Initialize hypervisor.
    ///
    /// # Panics
    /// It will be panic when parsing device tree failed.
    pub fn new(device_tree: Fdt) -> Self {
        const ARRAY_INIT_VALUE: Option<Guest> = None;
        HypervisorData {
            current_hart: 0,
            guests: [ARRAY_INIT_VALUE; MAX_HART_NUM],
            devices: Devices::new(device_tree),
        }
    }

    /// # Panics
    /// It will be panic if devices are uninitialized.
    #[must_use]
    pub fn devices(&mut self) -> &mut device::Devices {
        &mut self.devices
    }

    /// # Panics
    /// It will be panic if current HART's guest data is empty.
    pub fn guest(&self) -> &Guest {
        self.guests[self.current_hart]
            .as_ref()
            .expect("guest data not found")
    }

    /// # Panics
    /// It will be panic if `hart_id` is greater than `MAX_HART_NUM`.
    pub fn register_guest(&mut self, new_guest: Guest) {
        let hart_id = new_guest.hart_id();
        assert!(hart_id < MAX_HART_NUM);
        self.guests[hart_id] = Some(new_guest);
    }
}

/// Entry function. `__risc_v_rt__main` is alias of `__init` function in machine_init.rs.
/// * set stack pointer
/// * init mtvec and stvec
/// * jump to mstart
#[entry]
fn _start(hart_id: usize, dtb_addr: usize) -> ! {
    unsafe {
        // Initialize global allocator
        ALLOCATOR.lock().init(
            core::ptr::addr_of_mut!(_start_heap),
            core::ptr::addr_of!(_hv_heap_size) as usize,
        );
    }

    unsafe {
        // set stack pointer
        asm!(
            "
            mv a0, {hart_id}
            mv a1, {dtb_addr}
            mv t1, {stack_size_per_hart}
            mul t0, a0, t1
            mv sp, {stack_base}
            add sp, sp, t0
            csrw mtvec, {DRAM_BASE}
            csrw stvec, {DRAM_BASE}
            j {mstart}
            ",
            hart_id = in(reg) hart_id,
            dtb_addr = in(reg) dtb_addr,
            stack_size_per_hart = in(reg) STACK_SIZE_PER_HART,
            stack_base = in(reg) core::ptr::addr_of!(_top_m_stack) as usize,
            DRAM_BASE = in(reg) DRAM_BASE,
            mstart = sym mstart,
        );
    }

    unreachable!();
}
