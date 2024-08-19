mod exception;
mod interrupt;

use exception::trap_exception;
use interrupt::trap_interrupt;

use core::arch::asm;
use riscv::register::mcause::{self, Trap};

#[inline(always)]
#[allow(clippy::inline_always)]
unsafe fn mtrap_exit() -> ! {
    asm!(
        "
        li sp, 0x80200000 // MACHINE_STATIC_BASE + CONTEXT_OFFSET
        addi sp, sp, -256
        
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

        // revert stack pointer to 0x80200000
        addi sp, sp, 256

        // swap current sp for stored original mode sp
        csrrw sp, mscratch, sp

        mret
        ",
        options(noreturn),
    );
}

#[inline(always)]
#[allow(clippy::inline_always)]
unsafe fn mtrap_exit_sbi(error: usize, value: usize) -> ! {
    asm!("
        li sp, 0x80200000 // MACHINE_STATIC_BASE + CONTEXT_OFFSET
        addi sp, sp, -256

        ld ra, 1*8(sp)
        ld gp, 3*8(sp)
        ld tp, 4*8(sp)
        ld t0, 5*8(sp)
        ld t1, 6*8(sp)
        ld t2, 7*8(sp)
        ld s0, 8*8(sp)
        ld s1, 9*8(sp)
        mv a0, {error}
        mv a1, {value}
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

        // revert stack pointer to top (0x81800000)
        addi sp, sp, 256

        // swap current sp for stored original mode sp
        csrrw sp, mscratch, sp

        mret
        ",
        error = in(reg) error,
        value = in(reg) value,
        options(noreturn),
    );
}

/// Trap vector for M-mode.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn mtrap_vector() -> ! {
    asm!(
        ".align 4
        fence.i
        // swap original mode sp for machine mode sp
        csrrw sp, mscratch, sp

        // alloc register context region
        li sp, 0x80200000 // MACHINE_STATIC_BASE + CONTEXT_OFFSET
        addi sp, sp, -256

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
        ",
    );

    mtrap_vector2();
}

/// Separated from `mtrap_vector` by stack pointer circumstance.
#[no_mangle]
pub unsafe extern "C" fn mtrap_vector2() -> ! {
    match mcause::read().cause() {
        Trap::Interrupt(interrupt_cause) => trap_interrupt(interrupt_cause),
        Trap::Exception(exception_cause) => trap_exception(exception_cause),
    }
}
