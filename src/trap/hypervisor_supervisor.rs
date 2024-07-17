mod exception;
mod interrupt;

use exception::trap_exception;
use interrupt::trap_interrupt;

use crate::HYPERVISOR_DATA;
use core::arch::asm;
use riscv::register::mcause::{self, Trap};

#[no_mangle]
pub unsafe extern "C" fn hstrap_vector() -> ! {
    // save current context data
    HYPERVISOR_DATA.lock().context.store();

    let a0: u64 = 0;
    let a7: u64 = 0;
    asm!("ld {a0_reg}, 64(sp)", a0_reg = in(reg) a0);
    asm!("ld {a7_reg}, 120(sp)", a7_reg = in(reg) a7);
    match mcause::read().cause() {
        Trap::Interrupt(interrupt_cause) => trap_interrupt(interrupt_cause),
        Trap::Exception(exception_cause) => trap_exception(a0, a7, exception_cause),
    }

    // restore context data
    HYPERVISOR_DATA.lock().context.load();

    unsafe {
        asm!("sret", options(noreturn));
    }
}
