//! Trap VS-mode interrupt.

use super::hstrap_exit;
use crate::device::plic::ContextId;
use crate::h_extension::csrs::{hvip, VsInterruptKind};
use crate::HYPERVISOR_DATA;

use riscv::register::scause::Interrupt;
use riscv::register::sie;

/// Trap handler for Interrupt
#[allow(clippy::module_name_repetitions)]
pub fn trap_interrupt(interrupt_cause: Interrupt) -> ! {
    match interrupt_cause {
        Interrupt::SupervisorSoft => unsafe {
            hvip::set(VsInterruptKind::Software);
            sie::clear_ssoft();
        },
        Interrupt::SupervisorTimer => unsafe {
            hvip::set(VsInterruptKind::Timer);
            sie::clear_stimer();
        },
        Interrupt::SupervisorExternal => unsafe {
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
        Interrupt::Unknown => panic!("unknown interrupt type"),
    }

    unsafe {
        hstrap_exit();
    }
}
