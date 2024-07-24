use crate::guest;
use crate::h_extension::csrs::vstvec;
use crate::HYPERVISOR_DATA;
use core::arch::asm;
use riscv::register::scause;
use riscv::register::scause::Exception;

/// Delegate exception to supervisor mode from VS-mode.
#[no_mangle]
pub extern "C" fn hs_forward_exception() {
    unsafe {
        let mut context = HYPERVISOR_DATA.lock().guest.context;
        asm!(
            "csrw vsepc, {sepc}",
            "csrw vscause, {scause}",
            sepc = in(reg) context.sepc(),
            scause = in(reg) scause::read().bits()
        );

        context.set_sepc(vstvec::read().bits());

        // restore context data
        guest::context::load();
    }
}

/// Trap handler for exception
pub unsafe fn trap_exception(_exception_cause: Exception) -> ! {
    hs_forward_exception();

    unsafe {
        asm!("sret", options(noreturn));
    }
}
