#![no_main]
#![no_std]

extern crate panic_halt;
mod memmap;
mod supervisor_init;

use crate::memmap::{DRAM_BASE, STACK_BASE, STACK_SIZE_PER_HART};
use core::arch::{asm, global_asm};
use riscv::asm::sfence_vma_all;
use riscv::register::{medeleg, mepc, mideleg, mie, mscratch, mstatus, mtvec, satp};
use riscv_rt::entry;

global_asm!(include_str!("trap.S"));
extern "C" {
    fn trap_vector();
}

/// Start function
/// - set stack pointer
/// - init mtvec and stvec
/// - jump to mstart
#[entry]
fn _start(hart_id: usize, dtb_addr: usize) -> ! {
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

        mepc::write(supervisor_init::sstart as *const fn() as usize);

        // set trap_vector in trap.S to mtvec
        mtvec::write(trap_vector as *const fn() as usize, mtvec::TrapMode::Direct);

        sfence_vma_all();
    }

    enter_supervisor_mode(hart_id, dtb_addr);
}

/// Enter supervisor (just exec mret)
/// Jump to sstart
#[inline(never)]
fn enter_supervisor_mode(_hart_id: usize, _dtb_addr: usize) {
    unsafe {
        asm!("mret");
    }
}
