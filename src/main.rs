#![doc = include_str!("../README.md")]
#![no_main]
#![no_std]
// TODO: FIX AND REMOVE IT!!!
#![allow(static_mut_refs)]

mod hypervisor_init;
mod linker;
mod target_board;
mod trap;

use core::arch::naked_asm;
use core::panic::PanicInfo;

use linked_list_allocator::LockedHeap;

use hikami_core::println;
use hypervisor_init::hstart;
use linker::_top_b_stack;

/// Boot stack size for each HART.
const STACK_SIZE_PER_HART: usize = 0x1_0000;

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
/// - jump to hstart
///
/// TODO: Remove the `.attribute arch, "rv64gc"` directive when the LLVM problem is fixed.
#[unsafe(link_section = ".text.entry")]
#[unsafe(no_mangle)]
#[unsafe(naked)]
extern "C" fn _start() -> ! {
    // set stack pointer
    naked_asm!(
        r#"
            .attribute arch, "rv64gc"
            li t0, {stack_size_per_hart}
            mul t1, a0, t0
            la sp, {stack_top}
            sub sp, sp, t1

            call {hstart}
            "#,
        stack_top = sym _top_b_stack,
        stack_size_per_hart = const STACK_SIZE_PER_HART,
        hstart = sym hstart,
    )
}
