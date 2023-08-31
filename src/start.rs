#![no_main]
#![no_std]

extern crate panic_halt;
mod memmap;

use core::arch::asm;
use memmap::{DRAM_BASE, STACK_BASE, STACK_SIZE_PER_HART};
use riscv::asm::sfence_vma_all;
use riscv::register::{mcounteren, medeleg, mideleg, mie, mscratch, mstatus, mtvec, satp, stvec};
use riscv_rt::entry;

/// Start function
/// - set stack pointer
/// - init mtvec and stvec
/// - jump to mstart
#[entry]
fn _start(hart_id: usize, dtb_addr: usize) -> ! {
    unsafe {
        // set stack pointer
        asm!(
            "li sp, {}
            li t1, {}
            mul t0, a0, t1
            add sp, sp, t0",
            in(reg) STACK_BASE,
            in(reg) STACK_SIZE_PER_HART,
        );

        mtvec::write(DRAM_BASE as usize, mtvec::TrapMode::Direct);
        stvec::write(DRAM_BASE as usize, mtvec::TrapMode::Direct);
    }

    mstart(hart_id, dtb_addr);

    unreachable!();
}

/// Machine start function
fn mstart(hart_id: usize, dtb_addr: usize) {
    unsafe {
        // mideleg = 0x0222
        mideleg::set_sext();
        mideleg::set_ssoft();
        mideleg::set_stimer();
        // medeleg = 0xb1ff
        medeleg::set_instruction_misaligned();
        medeleg::set_instruction_fault();
        medeleg::set_illegal_instruction();
        medeleg::set_breakpoint();
        medeleg::set_load_misaligned();
        medeleg::set_load_fault();
        medeleg::set_store_misaligned();
        medeleg::set_store_fault();
        medeleg::set_user_env_call();
        medeleg::set_instruction_page_fault();
        medeleg::set_load_page_fault();
        medeleg::set_store_page_fault();
        // mie = 0x088
        mie::set_msoft();
        mie::set_mtimer();

        mstatus::set_mpp(mstatus::MPP::Supervisor);
        mscratch::write(STACK_BASE + STACK_SIZE_PER_HART * hart_id);
        satp::set(satp::Mode::Bare, 0, 0);

        mtvec::write(DRAM_BASE as usize, mtvec::TrapMode::Direct);

        sfence_vma_all();
    }
}
