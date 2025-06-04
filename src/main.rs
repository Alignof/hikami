#![doc = include_str!("../README.md")]
#![no_main]
#![no_std]
// TODO: remove nightly when `naked_functions` become stable.
#![feature(naked_functions)]
// TODO: FIX AND REMOVE IT!!!
#![allow(static_mut_refs)]

extern crate alloc;
mod device;
mod emulate_extension;
mod guest;
mod h_extension;
mod hypervisor_init;
mod log;
mod memmap;
mod trap;

use core::arch::naked_asm;
use core::panic::PanicInfo;

use linked_list_allocator::LockedHeap;

use crate::hypervisor_init::hstart;
use crate::memmap::constant::{DRAM_BASE, MAX_HART_NUM, STACK_SIZE_PER_HART};
use hikami::{HypervisorData, PageBlock, GUEST_DTB, GUEST_INITRD, GUEST_KERNEL, HYPERVISOR_DATA};
use hikami::{_end_bss, _hv_heap_size, _stack_start, _start_bss, _start_heap, _top_b_stack};

/// Panic handler
#[panic_handler]
pub fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {
        riscv::asm::wfi();
    }
}

#[global_allocator]
/// Global allocator.
static ALLOCATOR: LockedHeap = LockedHeap::empty();
// static mut ALLOCATOR: WildScreenAlloc = WildScreenAlloc::empty();

/// Entry function of the hypervisor.
///
/// - set stack pointer
/// - init stvec
/// - jump to hstart
///
/// TODO: Remove the `.attribute arch, "rv64gc"` directive when the LLVM problem is fixed.
#[link_section = ".text.entry"]
#[no_mangle]
#[naked]
extern "C" fn _start() -> ! {
    unsafe {
        // set stack pointer
        naked_asm!(
            r#"
            .attribute arch, "rv64gc"
            li t0, {stack_size_per_hart}
            mul t1, a0, t0
            la sp, {stack_top}
            sub sp, sp, t1

            li t2, {DRAM_BASE}
            csrw stvec, t2

            call {hstart}
            "#,
            stack_top = sym _top_b_stack,
            stack_size_per_hart = const STACK_SIZE_PER_HART,
            DRAM_BASE = const DRAM_BASE,
            hstart = sym hstart,
        )
    }
}
