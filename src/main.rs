#![doc = include_str!("../README.md")]
#![no_main]
#![no_std]
// TODO: remove nightly when `naked_functions` become stable.
#![feature(naked_functions)]
// TODO: FIX AND REMOVE IT!!!
#![allow(static_mut_refs)]

mod hypervisor_init;
mod trap;

use core::arch::naked_asm;
use core::panic::PanicInfo;

use linked_list_allocator::LockedHeap;

use crate::hypervisor_init::hstart;
use hikami_core::memmap::constant::{DRAM_BASE, STACK_SIZE_PER_HART};
use hikami_core::println;
use hikami_core::{_end_bss, _start_bss, _top_b_stack};

/// Guest kernel image
#[unsafe(link_section = ".guest_kernel")]
pub static GUEST_KERNEL: [u8; include_bytes!("../guest_image/vmlinux").len()] =
    *include_bytes!("../guest_image/vmlinux");

/// Device tree blob that is passed to guest
#[unsafe(link_section = ".guest_dtb")]
pub static GUEST_DTB: [u8; include_bytes!("../guest_image/guest.dtb").len()] =
    *include_bytes!("../guest_image/guest.dtb");

/// Guest intird
#[unsafe(link_section = ".guest_initrd")]
pub static GUEST_INITRD: [u8; include_bytes!("../guest_image/initrd").len()] =
    *include_bytes!("../guest_image/initrd");

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
