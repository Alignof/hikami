mod exception;
mod interrupt;

use exception::trap_exception;
use interrupt::trap_interrupt;

use core::arch::asm;
use riscv::register::mcause::{self, Trap};

#[inline(always)]
#[allow(clippy::inline_always)]
unsafe fn mtrap_entry() {
    asm!(
        ".align 4
        fence.i
        csrw mscratch, sp
        li sp, 0x80300000
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

        // store stack pointer
        csrr t0, mscratch
        sd t0, 2*8(sp)

        // restore sp
        csrr sp, mscratch
        ",
    );
}

#[inline(always)]
#[allow(clippy::inline_always)]
unsafe fn mtrap_exit() -> ! {
    asm!(
        "
        fence.i
        csrw mscratch, sp
        li sp, 0x80300000
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
        csrr sp, mscratch

        addi sp, sp, 256
        csrrw sp, mscratch, sp
        mret
        ",
        options(noreturn),
    );
}

#[inline(always)]
#[allow(clippy::inline_always)]
unsafe fn mtrap_exit_with_ret_value(ret_value: u64) -> ! {
    asm!("
        li sp, 0x80300000

        ld ra, 1*8(sp)
        ld gp, 3*8(sp)
        ld tp, 4*8(sp)
        ld t0, 5*8(sp)
        ld t1, 6*8(sp)
        ld t2, 7*8(sp)
        ld s0, 8*8(sp)
        ld s1, 9*8(sp)
        mv a0, {ret_value}
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

        addi sp, sp, 256
        csrrw sp, mscratch, sp
        mret
        ",
        ret_value = in(reg) ret_value,
        options(noreturn),
    );
}

#[no_mangle]
pub unsafe extern "C" fn mtrap_vector() -> ! {
    mtrap_entry();

    let a0: u64 = 0;
    let a6: u64 = 0;
    let a7: u64 = 0;
    asm!("
        ld a0, 10*8(sp)
        ld a6, 16*8(sp)
        ld a7, 17*8(sp)
        ",
        in("a0") a0,
        in("a6") a6,
        in("a7") a7
    );
    match mcause::read().cause() {
        Trap::Interrupt(interrupt_cause) => trap_interrupt(interrupt_cause),
        Trap::Exception(exception_cause) => trap_exception(a0, a6, a7, exception_cause),
    }
}
