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
mod util;

use core::arch::asm;
use core::cell::OnceCell;
use core::panic::PanicInfo;
use riscv_rt::entry;

use once_cell::unsync::Lazy;
use spin::Mutex;

use crate::guest::Guest;
use crate::machine_init::mstart;
use crate::memmap::constant::{machine, DRAM_BASE, MAX_HART_NUM, STACK_SIZE_PER_HART};
use crate::sbi::Sbi;

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
#[derive(Debug, Default)]
pub struct HypervisorData {
    current_hart: usize,
    guest: [Option<guest::Guest>; MAX_HART_NUM],
    devices: Option<device::Devices>,
}

impl HypervisorData {
    /// # Panics
    /// It will be panic if devices are uninitialized.
    #[must_use]
    pub fn devices(&self) -> &device::Devices {
        self.devices.as_ref().expect("device data is uninitialized")
    }

    /// # Panics
    /// It will be panic if current HART's guest data is empty.
    pub fn guest(&mut self) -> &mut Guest {
        self.guest[self.current_hart]
            .as_mut()
            .expect("guest data not found")
    }

    /// # Panics
    /// It will be panic if `hart_id` is greater than `MAX_HART_NUM`.
    pub fn register_guest(&mut self, new_guest: Guest) {
        let hart_id = new_guest.hart_id();
        assert!(hart_id < MAX_HART_NUM);
        self.guest[hart_id] = Some(new_guest);
    }
}

use linked_list_allocator::LockedHeap;
#[global_allocator]
// static mut ALLOCATOR: WildScreenAlloc = WildScreenAlloc::empty();
static ALLOCATOR: LockedHeap = LockedHeap::empty();

/// TODO: change to `Mutex<OnceCell<HypervisorData>>`?
static mut HYPERVISOR_DATA: Lazy<Mutex<HypervisorData>> =
    Lazy::new(|| Mutex::new(HypervisorData::default()));

/// Singleton for SBI handler.
static SBI: Mutex<OnceCell<Sbi>> = Mutex::new(OnceCell::new());

/// Device tree blob that is passed to guest
#[link_section = ".guest_dtb"]
static GUEST_DTB: [u8; include_bytes!("../guest.dtb").len()] = *include_bytes!("../guest.dtb");

extern "C" {
    /// start of heap (defined in `memory.x`)
    static mut _start_heap: u8;
    /// heap size (defined in `memory.x`)
    static _hv_heap_size: u8;
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
            &mut _start_heap as *mut u8,
            &_hv_heap_size as *const u8 as usize,
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
            stack_base = in(reg) machine::STACK_BASE.raw(),
            DRAM_BASE = in(reg) DRAM_BASE,
            mstart = sym mstart,
        );
    }

    unreachable!();
}
