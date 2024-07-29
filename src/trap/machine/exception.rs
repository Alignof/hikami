use super::{mtrap_exit, mtrap_exit_with_ret_value};
use crate::SBI;
use riscv::register::mcause::Exception;
use riscv::register::{mcause, mepc, mstatus, mtval, scause, sepc, stval, stvec};
use rustsbi::RustSBI;

/// Trap SBI Ecall
///
/// Handling SBI ecall is delegated to `Sbi` struct.
pub unsafe fn trap_envcall(a0: usize, a1: usize, a2: usize, a6: usize, a7: usize) -> ! {
    let ret_val = SBI
        .lock()
        .get()
        .unwrap()
        .handle_ecall(a7, a6, [a0, a1, a2, 0, 0, 0]);

    if ret_val.error == 0 {
        mtrap_exit_with_ret_value(ret_val.value);
    } else {
        panic!(
            "SBI call failed: error:{}, eid:{a7}, fid:{a6}",
            ret_val.error
        );
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
pub unsafe fn trap_exception(
    a0: usize,
    a1: usize,
    a2: usize,
    a6: usize,
    a7: usize,
    exception_cause: Exception,
) -> ! {
    if exception_cause == Exception::UserEnvCall {
        trap_envcall(a0, a1, a2, a6, a7);
    } else {
        forward_exception();
        mtrap_exit();
    }
}
