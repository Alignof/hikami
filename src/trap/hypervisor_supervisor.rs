mod exception;
mod interrupt;

use exception::trap_exception;
use interrupt::trap_interrupt;

use crate::guest;
use core::arch::asm;
use riscv::register::scause::{self, Trap};

/// Switch to hypervisor stack and save contexts.
///
/// # TODO
/// replace stringify macro to const when `asm_const` is stabled.
#[inline(always)]
#[allow(clippy::inline_always)]
unsafe fn hstrap_entry() {
    asm!(
        ".align 4
        fence.i
        // swap original mode sp for HS-mode sp 
        csrrw sp, sscratch, sp
        addi sp, sp, -260

        // save registers
        sd ra, 1*8(sp)
        sd gp, 3*8(sp)
        sd tp, 4*8(sp)
        sd t0, 5*8(sp)
        sd t1, 6*8(sp)
        sd t2, 7*8(sp)
        sd s0, 8*8(sp)
        sd s1, 9*8(sp)
        sd a0, 10*8(sp)
        sd a1, 11*8(sp)
        sd a2, 12*8(sp)
        sd a3, 13*8(sp)
        sd a4, 14*8(sp)
        sd a5, 15*8(sp)
        sd a6, 16*8(sp)
        sd a7, 17*8(sp)
        sd s2, 18*8(sp)
        sd s3, 19*8(sp)
        sd s4, 20*8(sp)
        sd s5, 21*8(sp)
        sd s6, 22*8(sp)
        sd s7, 23*8(sp)
        sd s8, 24*8(sp)
        sd s9, 25*8(sp)
        sd s10, 26*8(sp)
        sd s11, 27*8(sp)
        sd t3, 28*8(sp)
        sd t4, 29*8(sp)
        sd t5, 30*8(sp)
        sd t6, 31*8(sp)

        // save sstatus
        csrr t0, sstatus
        sd t0, 32*8(sp)

        // save pc
        csrr t1, sepc
        sd t1, 33*8(sp)
        ",
    );
}

/// Switch to original mode stack and save contexts.
///
/// # TODO
/// replace stringify macro to const when `asm_const` is stabled.
#[inline(always)]
#[allow(clippy::inline_always)]
unsafe fn hstrap_exit() {
    asm!(
        ".align 4
        fence.i

        // restore sstatus 
        ld t0, 32*8(sp)
        csrw sstatus, t0

        // restore pc
        ld t1, 33*8(sp)
        csrw sepc, t1

        // restore registers
        ld ra, 1*8(sp)
        ld gp, 3*8(sp)
        ld tp, 4*8(sp)
        ld t0, 5*8(sp)
        ld t1, 6*8(sp)
        ld t2, 7*8(sp)
        ld s0, 8*8(sp)
        ld s1, 9*8(sp)
        ld a0, 10*8(sp)
        ld a1, 11*8(sp)
        ld a2, 12*8(sp)
        ld a3, 13*8(sp)
        ld a4, 14*8(sp)
        ld a5, 15*8(sp)
        ld a6, 16*8(sp)
        ld a7, 17*8(sp)
        ld s2, 18*8(sp)
        ld s3, 19*8(sp)
        ld s4, 20*8(sp)
        ld s5, 21*8(sp)
        ld s6, 22*8(sp)
        ld s7, 23*8(sp)
        ld s8, 24*8(sp)
        ld s9, 25*8(sp)
        ld s10, 26*8(sp)
        ld s11, 27*8(sp)
        ld t3, 28*8(sp)
        ld t4, 29*8(sp)
        ld t5, 30*8(sp)
        ld t6, 31*8(sp)

        // swap HS-mode sp for original mode sp.
        addi sp, sp, 260
        csrrw sp, sscratch, sp
        ",
    );
}

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
    unsafe {
        asm!(
            "
            .align 4

            // switch stack pointer for HS-mode
            csrrw sp, sscratch, sp
            "
        );
    }

    // save current context data
    guest::context::store();

    match scause::read().cause() {
        Trap::Interrupt(interrupt_cause) => trap_interrupt(interrupt_cause),
        Trap::Exception(exception_cause) => trap_exception(exception_cause),
    }
}
