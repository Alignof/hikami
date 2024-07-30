mod sbi;

use crate::guest;
use crate::h_extension::csrs::vstvec;
use crate::HYPERVISOR_DATA;
use core::arch::asm;
use riscv::register::scause;
use riscv::register::scause::Exception;
use sbi::sbi_base_handler;

/// Delegate exception to supervisor mode from VS-mode.
#[no_mangle]
#[inline(always)]
#[allow(clippy::inline_always)]
pub extern "C" fn hs_forward_exception() {
    unsafe {
        let mut context = HYPERVISOR_DATA.lock().guest().context;
        asm!(
            "csrw vsepc, {sepc}",
            "csrw vscause, {scause}",
            sepc = in(reg) context.sepc(),
            scause = in(reg) scause::read().bits()
        );

        context.set_sepc(vstvec::read().bits());
    }
}

/// Handler for Ecall from VS-mode exception
fn sbi_vs_mode_handler(context: &mut guest::context::Context) {
    let ext_id: usize = context.xreg(17) as usize;
    let func_id: usize = context.xreg(16) as usize;

    let sbiret = match ext_id {
        sbi_spec::base::EID_BASE => sbi_base_handler(func_id),
        _ => panic!(
            "Unsupported SBI call, eid: {:x}, fid: {:x}",
            ext_id, func_id
        ),
    };

    context.set_xreg(10, sbiret.error as u64);
    context.set_xreg(11, sbiret.value as u64);
}

/// Trap handler for exception
pub unsafe fn trap_exception(exception_cause: Exception) -> ! {
    let mut context = unsafe { HYPERVISOR_DATA.lock().guest().context };

    match exception_cause {
        Exception::SupervisorEnvCall => panic!("SupervisorEnvCall should be handled by M-mode"),
        // Enum not found in `riscv` crate.
        Exception::Unknown => {
            match scause::read().code() {
                // Ecall from VS-mode
                10 => {
                    sbi_vs_mode_handler(&mut context);
                    context.set_sepc(context.sepc() + 4);
                }
                _ => unreachable!(),
            }
        }
        _ => hs_forward_exception(),
    }

    // restore context data
    guest::context::load();

    unsafe {
        asm!("sret", options(noreturn));
    }
}
