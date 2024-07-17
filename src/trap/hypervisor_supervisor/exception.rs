use crate::h_extension::csrs::vstvec;
use crate::HYPERVISOR_DATA;
use core::arch::asm;
use riscv::register::scause;
use riscv::register::scause::Exception;

/// Delegate exception to supervisor mode from VS-mode.
#[no_mangle]
pub extern "C" fn hs_forward_exception() {
    unsafe {
        let context = &mut HYPERVISOR_DATA.lock().context;
        asm!(
            "csrw vsepc, {sepc}",
            "csrw vscause, {scause}",
            sepc = in(reg) context.sepc,
            scause = in(reg) scause::read().bits()
        );

        context.set_sepc(vstvec::read().bits());
    }
}

/// Trap handler for exception
#[allow(clippy::cast_possible_wrap)]
pub unsafe fn trap_exception(_a0: u64, _a7: u64, _exception_cause: Exception) {
    hs_forward_exception();
}
