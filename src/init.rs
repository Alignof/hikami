#![no_main]
#![no_std]

extern crate panic_halt;
mod memmap;
mod start;
mod uart;

use crate::memmap::{DRAM_BASE, PA2VA_OFFSET, PAGE_TABLE_BASE, PAGE_TABLE_SIZE, STACK_BASE};
use core::arch::asm;
use riscv::asm::sfence_vma;
use riscv::register::{mtvec, satp, stvec};
use riscv_rt::entry;

/// entry point  
/// Initialize CSRs, page tables, stack pointer
#[entry]
fn init() -> ! {
    let hart_id: u64;
    let dtb_addr: u64;
    unsafe {
        // get hart id
        asm!("mv {}, a0", out(reg) hart_id);
        asm!("mv {}, a1", out(reg) dtb_addr);
    }

    // init stack pointer
    let stack_pointer = STACK_BASE + PA2VA_OFFSET;
    unsafe {
        asm!("mv sp, {}", in(reg) stack_pointer);
    }

    // init page tables
    let init_func = __risc_v_rt__main;
    let offset_from_dram_base = init_func as *const fn() as u64 - DRAM_BASE;
    let offset_from_dram_base_masked = (offset_from_dram_base >> 21) << 19;
    let page_table_start = PAGE_TABLE_BASE + offset_from_dram_base + hart_id * PAGE_TABLE_SIZE;
    for pt_index in 511..1024 {
        let pt_offset = (page_table_start + pt_index * 8) as *mut u64;
        unsafe {
            pt_offset.write_volatile(pt_offset.read_volatile() + offset_from_dram_base_masked);
        }
    }

    unsafe {
        // init trap vector
        stvec::write(trampoline as *const fn() as usize, mtvec::TrapMode::Direct);

        // set satp(Supervisor Address Translation and Protection) register
        satp::set(satp::Mode::Sv39, 0, (page_table_start >> 12) as usize);

        // sfence.vma
        sfence_vma(0, 0);
    }

    // jump to trampoline
    trampoline(hart_id, dtb_addr);

    unreachable!()
}

/// Jump to start
pub fn trampoline(hart_id: u64, dtb_addr: u64) {
    start::start(hart_id, dtb_addr);
}
