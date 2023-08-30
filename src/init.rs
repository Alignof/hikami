#![no_main]
#![no_std]

extern crate panic_halt;
use core::arch::asm;
use riscv::asm::sfence_vma;
use riscv::register::{mtvec, satp, stvec};
use riscv_rt::entry;

const DRAM_BASE: u64 = 0x8000_0000;
const PAGE_TABLE_BASE: u64 = 0x8020_0000;
const PAGE_TABLE_SIZE: u64 = 1024;
const STACK_BASE: u64 = 0x8030_0000;
const PA2VA_OFFSET: u64 = 0xffff_ffff_4000_0000;

/// entry point  
/// Initialize CSRs, page tables, stack pointer
#[entry]
fn init() -> ! {
    let hart_id: u64;
    unsafe {
        // get hart id
        asm!("mv {}, a0", out(reg) hart_id);

        // debug output
        let uart = 0x1000_0000 as *mut u32;
        for c in b"hart_id: ".iter() {
            while (uart.read_volatile() as i32) < 0 {}
            uart.write_volatile(*c as u32);
        }
        uart.write_volatile(hart_id as u32 + '0' as u32);
        uart.write_volatile('\n' as u32);
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
    trampoline();

    unreachable!()
}

/// Jump to start
pub fn trampoline() {}
