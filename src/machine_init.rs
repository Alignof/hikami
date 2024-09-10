//! M-mode level initialization.

use crate::hypervisor_init;
use crate::memmap::constant::STACK_SIZE_PER_HART;
use crate::trap::machine::mtrap_vector;
use crate::{sbi::Sbi, SBI};
use core::arch::asm;
use riscv::asm::sfence_vma_all;
use riscv::register::{
    mcounteren, medeleg, mepc, mideleg, mie, mscratch, mstatus, mtvec, pmpaddr0, pmpaddr1,
    pmpaddr2, pmpcfg0, satp, Permission, Range,
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
        asm!("csrs medeleg, {vsmode_ecall}", vsmode_ecall = in(reg) 1 << 10, options(nomem)); // deleg env call from VS-mode
        asm!("csrs medeleg, {load_guest_page_fault}", load_guest_page_fault = in(reg) 1 << 21, options(nomem)); // deleg load guest page fault
        asm!("csrs medeleg, {virtual_instruction}", virtual_instruction = in(reg) 1 << 22, options(nomem)); // deleg virtual instruction
        asm!("csrs medeleg, {store_amo_guest_page_fault}", store_amo_guest_page_fault = in(reg) 1 << 23, options(nomem)); // deleg store/amo guest page fault
        medeleg::clear_supervisor_env_call();

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

        // switch to S-mode when mret executed.
        mstatus::set_mpp(mstatus::MPP::Supervisor);

        // set M-mode stack pointer
        mscratch::write(
            core::ptr::addr_of!(crate::_top_m_stack) as usize + STACK_SIZE_PER_HART * hart_id,
        );

        // pmp settings
        pmpcfg0::set_pmp(0, Range::OFF, Permission::NONE, false);
        pmpaddr0::write(0);
        // 0x0 - 0x8000_0000 = RW
        pmpcfg0::set_pmp(1, Range::TOR, Permission::RW, false);
        pmpaddr1::write(0x8000_0000 >> 2);
        // 0x8000_0000 - 0xffff_ffff = RWX
        pmpcfg0::set_pmp(2, Range::TOR, Permission::RWX, false);
        pmpaddr2::write(0xffff_ffff);

        // no address translation
        satp::set(satp::Mode::Bare, 0, 0);

        // enable Sstc and Zicboz extention
        asm!("csrs menvcfg, {sstc_cbze}", sstc_cbze = in(reg) (1u64 << 63) | (1u64 << 7) | (1u64 << 6), options(nomem)); // deleg env call from VS-mode

        // set `hstart` to jump after mret
        mepc::write(hypervisor_init::hstart as *const fn() as usize);

        // set trap_vector in trap.S to mtvec
        mtvec::write(
            mtrap_vector as *const fn() as usize,
            mtvec::TrapMode::Direct,
        );

        sfence_vma_all();
    }

    SBI.lock().get_or_init(|| {
        // parse device tree
        let device_tree = unsafe {
            match fdt::Fdt::from_ptr(dtb_addr as *const u8) {
                Ok(fdt) => fdt,
                Err(e) => panic!("{}", e),
            }
        };

        Sbi::new(device_tree)
    });

    enter_hypervisor_mode(hart_id, dtb_addr);
}

/// Enter hypervisor. (just exec mret)
///
/// Jump to hstart via mret.
#[inline(never)]
#[no_mangle]
extern "C" fn enter_hypervisor_mode(hart_id: usize, dtb_addr: usize) -> ! {
    unsafe {
        // set stack pointer
        asm!(
            "
            mv t0, {hart_id}
            mv t1, {dtb_addr}
            mv sp, {machine_sp}
            ",
            hart_id = in(reg) hart_id,
            dtb_addr = in(reg) dtb_addr,
            machine_sp = in(reg) core::ptr::addr_of!(crate::_top_m_stack) as usize + STACK_SIZE_PER_HART * hart_id
        );
        // enter HS-mode.
        asm!(
            "
            mv a0, t0
            mv a1, t1
            mret
            ",
            options(noreturn)
        );
    }
}
