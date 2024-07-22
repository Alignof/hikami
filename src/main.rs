#![no_main]
#![no_std]

extern crate alloc;
mod guest;
mod h_extension;
mod hypervisor_init;
mod machine_init;
mod memmap;
mod trap;
mod util;

use core::arch::asm;
use core::panic::PanicInfo;
use riscv_rt::entry;
use wild_screen_alloc::WildScreenAlloc;

use once_cell::unsync::Lazy;
use spin::Mutex;

use crate::machine_init::mstart;
use crate::memmap::constant::{DRAM_BASE, HEAP_BASE, HEAP_SIZE, STACK_BASE, STACK_SIZE_PER_HART};

/// Panic handler
#[panic_handler]
pub fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {
        unsafe {
            riscv::asm::wfi();
        }
    }
}

/// Global data for hypervisor.
///
/// FIXME: Rename me!
#[derive(Debug, Default)]
pub struct HypervisorData {
    guest: guest::Guest,
}

#[global_allocator]
static mut ALLOCATOR: WildScreenAlloc = WildScreenAlloc::empty();

static mut HYPERVISOR_DATA: Lazy<Mutex<HypervisorData>> =
    Lazy::new(|| Mutex::new(HypervisorData::default()));

/// Entry function. `__risc_v_rt__main` is alias of `__init` function in machine_init.rs.
/// * set stack pointer
/// * init mtvec and stvec
/// * jump to mstart
#[entry]
fn _start(hart_id: usize, dtb_addr: usize) -> ! {
    unsafe {
        // Initialize global allocator
        ALLOCATOR.init(HEAP_BASE, HEAP_SIZE);
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
            stack_base = in(reg) STACK_BASE,
            DRAM_BASE = in(reg) DRAM_BASE,
            mstart = sym mstart,
        );
    }

    unreachable!();
}
