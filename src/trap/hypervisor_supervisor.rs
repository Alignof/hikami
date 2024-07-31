mod exception;
mod interrupt;

use exception::trap_exception;
use interrupt::trap_interrupt;

use crate::guest;
use core::arch::asm;
use riscv::register::scause::{self, Trap};

/// Trap vector for HS-mode.
///
/// TODO: function alignment (feature `fn_align`).  
/// See: [https://github.com/rust-lang/rust/issues/82232](https://github.com/rust-lang/rust/issues/82232).
/// ```no_run
/// #[repr(align(4))]
/// pub unsafe extern "C" fn hstrap_vector() -> ! { }
/// ```
#[no_mangle]
pub unsafe extern "C" fn hstrap_vector() -> ! {
    unsafe { asm!(".align 4") }

    // save current context data
    guest::context::store();

    match scause::read().cause() {
        Trap::Interrupt(interrupt_cause) => trap_interrupt(interrupt_cause),
        Trap::Exception(exception_cause) => trap_exception(exception_cause),
    }
}
