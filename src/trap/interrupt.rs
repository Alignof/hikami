//! Trap VS-mode interrupt.

use super::hstrap_exit;
use hikami_core::HYPERVISOR_DATA;
use hikami_core::device::plic::ContextId;
use hikami_core::h_extension::csrs::{VsInterruptKind, hvip};

use riscv::interrupt::{InterruptNumber, supervisor::Interrupt};
use riscv::register::sie;

/// Trap handler for Interrupt
#[allow(clippy::module_name_repetitions)]
pub fn trap_interrupt(interrupt_code: usize) -> ! {
    match Interrupt::from_number(interrupt_code) {
        Ok(Interrupt::SupervisorSoft) => unsafe {
            hvip::set(VsInterruptKind::Software);
            sie::clear_ssoft();
        },
        Ok(Interrupt::SupervisorTimer) => unsafe {
            hvip::set(VsInterruptKind::Timer);
            sie::clear_stimer();
        },
        Ok(Interrupt::SupervisorExternal) => unsafe {
            let mut hypervisor_data = HYPERVISOR_DATA.lock();
            let hart_id = hypervisor_data.get().unwrap().guest().hart_id();
            let context_id = ContextId::new(hart_id, true);

            // read plic claim/update register and reflect to plic.claim_complete.
            hypervisor_data
                .get_mut()
                .unwrap()
                .devices()
                .plic
                .update_claim_complete(&context_id);

            hvip::set(VsInterruptKind::External);
            sie::clear_sext();
        },
        Err(err_int) => panic!("unknown interrupt type: {err_int}"),
    }

    unsafe {
        hstrap_exit();
    }
}
