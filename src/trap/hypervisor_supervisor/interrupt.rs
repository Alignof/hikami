use super::hstrap_exit;
use crate::h_extension::csrs::{hvip, vsip, InterruptKind};
use crate::memmap::constant::device::CLINT_ADDR;
use crate::HYPERVISOR_DATA;
use riscv::register::scause::Interrupt;
use riscv::register::sie;

/// Trap handler for Interrupt
#[allow(clippy::module_name_repetitions)]
pub unsafe fn trap_interrupt(interrupt_cause: Interrupt) -> ! {
    match interrupt_cause {
        Interrupt::SupervisorSoft => {
            let hart_id = HYPERVISOR_DATA.lock().guest().hart_id();
            vsip::set_ssoft();
            let interrupt_addr = (CLINT_ADDR + hart_id * 4) as *mut u64;
            interrupt_addr.write_volatile(0);
        }
        Interrupt::SupervisorTimer => {
            hvip::set(InterruptKind::Vsti);
            sie::clear_stimer();
        }
        Interrupt::SupervisorExternal => riscv::asm::wfi(), // wait for interrupt
        _ => panic!("unknown interrupt type"),
    }

    hstrap_exit();
}
