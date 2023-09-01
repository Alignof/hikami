#![no_main]
#![no_std]

extern crate panic_halt;
mod memmap;

use core::arch::{asm, global_asm};
use memmap::{DRAM_BASE, STACK_BASE, STACK_SIZE_PER_HART};
use riscv::asm::sfence_vma_all;
use riscv::register::{medeleg, mideleg, mie, mscratch, mstatus, mtvec, satp, stvec};
use riscv_rt::entry;

global_asm!(include_str!("trap.S"));

/// Start function
/// - set stack pointer
/// - init mtvec and stvec
/// - jump to mstart
#[entry]
fn _start(hart_id: usize, dtb_addr: usize) -> ! {
    unsafe {
        // set stack pointer
        asm!(
            "mv sp, {}
            mv t1, {}
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

        // set trap_vector in trap.S to mtvec
        asm!("lla t0, trap_vector");
        asm!("csrw mtvec, t0");

        sfence_vma_all();
    }

    enter_supervisor_mode(hart_id, dtb_addr);
}

/// Enter supervisor (just exec mret)
#[inline(never)]
fn enter_supervisor_mode(_hart_id: usize, _dtb_addr: usize) {
    unsafe {
        asm!("mret");
    }
}
