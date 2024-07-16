use super::{mtrap_exit, mtrap_exit_with_ret_value};
use riscv::register::mcause::Exception;
use riscv::register::{mcause, mepc, mhartid, mip, mstatus, mtval, scause, sepc, stval, stvec};

pub unsafe fn trap_envcall(a0: u64, a6: u64, a7: u64) -> ! {
    const MTIMECMP_ADDR: usize = 0x200_4000;

    // https://doxygen.coreboot.org/d6/dfc/sbi_8c_source.html
    mepc::write(mepc::read() + 4);

    let (eid, fid) = (a7, a6);
    match (eid, fid) {
        // sbi_set_timer
        (0x0..=0xf, 0x0) => {
            // timer_value = a0
            let timer_value: u64 = a0;

            let mtimecmp_addr = (MTIMECMP_ADDR + mhartid::read() * 8) as *mut u64;
            mtimecmp_addr.write_volatile(timer_value);

            mtrap_exit_with_ret_value(0);
        }
        // sbi_clear_ipi
        (0x0..=0xf, 0x3) => {
            mip::clear_ssoft();

            mtrap_exit_with_ret_value(0);
        }
        // sbi_send_ipi
        (0x0..=0xf, 0x4) => {
            // mask_addr = a0
            let mask_addr: *mut u64 = a0 as *mut u64;
            let mut mask = if mstatus::read().mprv() {
                mask_addr.read_volatile()
            } else {
                mstatus::set_mprv();
                let mask = mask_addr.read_volatile();
                mstatus::clear_mprv();
                mask
            };

            let mut clint_addr: *mut u8 = 0x200_0000 as *mut u8;
            while mask != 0 {
                if mask & 1 == 1 {
                    clint_addr.write_volatile(1);
                }
                clint_addr = clint_addr.add(4);
                mask >>= 1;
            }

            mtrap_exit_with_ret_value(0);
        }
        // sbi_shutdown
        (0x0..=0xf, 0x8) => panic!("sbi shutdown"),
        // other
        _ => panic!("unknown ecall number"),
    }
}

/// Delegate exception to supervisor or user mode from machine mode.
#[no_mangle]
pub extern "C" fn forward_exception() {
    unsafe {
        sepc::write(mepc::read());
        scause::write(mcause::read().bits());
        stval::write(mtval::read());
        mepc::write(stvec::read().bits() & !0x3);

        if mstatus::read().sie() {
            mstatus::set_spie();
        } else {
            // clear?
        }

        if mstatus::read().mpp() == mstatus::MPP::Supervisor {
            mstatus::set_spp(mstatus::SPP::Supervisor);
        } else {
            mstatus::set_spp(mstatus::SPP::User);
        }

        mstatus::clear_sie();
        mstatus::set_mpp(mstatus::MPP::Supervisor);
    }
}

/// Trap handler for exception
#[allow(clippy::cast_possible_wrap)]
pub unsafe fn trap_exception(a0: u64, a6: u64, a7: u64, exception_cause: Exception) -> ! {
    if exception_cause == Exception::UserEnvCall {
        trap_envcall(a0, a6, a7);
    } else {
        forward_exception();
        mtrap_exit();
    }
}
