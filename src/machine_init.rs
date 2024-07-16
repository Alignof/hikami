use crate::hypervisor_init;
use crate::memmap::constant::{STACK_BASE, STACK_SIZE_PER_HART};
use crate::trap::machine::mtrap_vector;
use core::arch::asm;
use riscv::asm::sfence_vma_all;
use riscv::register::{
    mcounteren, medeleg, mepc, mideleg, mie, mscratch, mstatus, mtvec, pmpaddr0, pmpcfg0, satp,
};

/// Machine start function
pub fn mstart(hart_id: usize, dtb_addr: usize) -> ! {
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

        // mcounteren = 0xffff_ffff
        mcounteren::set_cy();
        mcounteren::set_tm();
        mcounteren::set_ir();
        mcounteren::set_hpm(3);
        mcounteren::set_hpm(4);
        mcounteren::set_hpm(5);
        mcounteren::set_hpm(6);
        mcounteren::set_hpm(7);
        mcounteren::set_hpm(8);
        mcounteren::set_hpm(9);
        mcounteren::set_hpm(10);
        mcounteren::set_hpm(11);
        mcounteren::set_hpm(12);
        mcounteren::set_hpm(13);
        mcounteren::set_hpm(14);
        mcounteren::set_hpm(15);
        mcounteren::set_hpm(16);
        mcounteren::set_hpm(17);
        mcounteren::set_hpm(18);
        mcounteren::set_hpm(19);
        mcounteren::set_hpm(20);
        mcounteren::set_hpm(21);
        mcounteren::set_hpm(22);
        mcounteren::set_hpm(23);
        mcounteren::set_hpm(24);
        mcounteren::set_hpm(25);
        mcounteren::set_hpm(26);
        mcounteren::set_hpm(27);
        mcounteren::set_hpm(28);
        mcounteren::set_hpm(29);
        mcounteren::set_hpm(30);
        mcounteren::set_hpm(31);
        mstatus::set_mpp(mstatus::MPP::Supervisor);
        mscratch::write(STACK_BASE + STACK_SIZE_PER_HART * hart_id);
        pmpaddr0::write(0xffff_ffff_ffff_ffff);
        pmpcfg0::write(pmpcfg0::read().bits | 0x1f);
        satp::set(satp::Mode::Bare, 0, 0);

        mepc::write(hypervisor_init::hstart as *const fn() as usize);

        // set trap_vector in trap.S to mtvec
        mtvec::write(
            mtrap_vector as *const fn() as usize,
            mtvec::TrapMode::Direct,
        );

        sfence_vma_all();
    }

    enter_hypervisor_mode(hart_id, dtb_addr);
}

/// Enter hypervisor. (just exec mret)
///
/// Jump to hstart via mret.
#[inline(never)]
#[no_mangle]
extern "C" fn enter_hypervisor_mode(_hart_id: usize, _dtb_addr: usize) -> ! {
    unsafe {
        asm!("mret", options(noreturn));
    }
}
