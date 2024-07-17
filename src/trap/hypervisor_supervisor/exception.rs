use riscv::register::mcause::Exception;
use riscv::register::{mcause, mepc, mstatus, mtval, scause, sepc, stval, stvec};

/// Delegate exception to supervisor mode from VS-mode.
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
pub unsafe fn trap_exception(_a0: u64, _a7: u64, _exception_cause: Exception) {
    forward_exception();
}
