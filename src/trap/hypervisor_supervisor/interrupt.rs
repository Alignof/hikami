//! Trap VS-mode interrupt.

use super::hstrap_exit;
use crate::device::Device;
use crate::h_extension::csrs::{hvip, vsip, VsInterruptKind};
use crate::HYPERVISOR_DATA;
use riscv::register::scause::Interrupt;
use riscv::register::sie;

/// Trap handler for Interrupt
#[allow(clippy::module_name_repetitions)]
pub unsafe fn trap_interrupt(interrupt_cause: Interrupt) -> ! {
    match interrupt_cause {
        Interrupt::SupervisorSoft => {
            let mut hypervisor_data = HYPERVISOR_DATA.lock();
            let hart_id = hypervisor_data.guest().hart_id();
            let clint_addr = hypervisor_data.devices.as_ref().unwrap().clint.paddr();

            vsip::set_ssoft();
            let interrupt_addr = (clint_addr.raw() + hart_id * 4) as *mut u64;
            interrupt_addr.write_volatile(0);
        }
        Interrupt::SupervisorTimer => {
            hvip::set(VsInterruptKind::Timer);
            sie::clear_stimer();
        }
        Interrupt::SupervisorExternal => hvip::set(VsInterruptKind::External),
        Interrupt::Unknown => panic!("unknown interrupt type"),
    }

    hstrap_exit();
}
