use super::{trap_exception, trap_interrupt};
use crate::memmap::constant::STACK_BASE;
use core::arch::asm;
use riscv::register::mcause;
use riscv::register::mcause::Trap;

#[no_mangle]
pub unsafe fn mtrap_vector() {
    asm!(
        ".align 4
        csrrw sp, mscratch, sp
        mv sp, {stack_base}

        addi sp, sp, -240
        sd ra, 0(sp)
        sd t0, 8(sp)
        sd t1, 16(sp)
        sd t2, 24(sp)
        sd t3, 32(sp)
        sd t4, 40(sp)
        sd t5, 48(sp)
        sd t6, 56(sp)
        sd a0, 64(sp)
        sd a1, 72(sp)
        sd a2, 80(sp)
        sd a3, 88(sp)
        sd a4, 96(sp)
        sd a5, 104(sp)
        sd a6, 112(sp)
        sd a7, 120(sp)
        sd s2, 128(sp)
        sd s3, 136(sp)
        sd s4, 144(sp)
        sd s5, 152(sp)
        sd s6, 160(sp)
        sd s7, 168(sp)
        sd s8, 176(sp)
        sd s9, 184(sp)
        sd s10, 192(sp)
        sd s11, 200(sp)
        sd t3, 208(sp)
        sd t4, 216(sp)
        sd t5, 224(sp)
        sd t6, 232(sp)
        ",
        stack_base = in(reg) STACK_BASE
    );

    let a0: u64 = 0;
    let a7: u64 = 0;
    asm!("ld {a0_reg}, 64(sp)", a0_reg = in(reg) a0);
    asm!("ld {a7_reg}, 120(sp)", a7_reg = in(reg) a7);
    match mcause::read().cause() {
        Trap::Interrupt(interrupt_cause) => trap_interrupt(interrupt_cause),
        Trap::Exception(exception_cause) => trap_exception(a0, a7, exception_cause),
    }

    asm!(
        "
        ld ra, 0(sp)
        ld t0, 8(sp)
        ld t1, 16(sp)
        ld t2, 24(sp)
        ld t3, 32(sp)
        ld t4, 40(sp)
        ld t5, 48(sp)
        ld t6, 56(sp)
        ld a0, 64(sp)
        ld a1, 72(sp)
        ld a2, 80(sp)
        ld a3, 88(sp)
        ld a4, 96(sp)
        ld a5, 104(sp)
        ld a6, 112(sp)
        ld a7, 120(sp)
        ld s2, 128(sp)
        ld s3, 136(sp)
        ld s4, 144(sp)
        ld s5, 152(sp)
        ld s6, 160(sp)
        ld s7, 168(sp)
        ld s8, 176(sp)
        ld s9, 184(sp)
        ld s10, 192(sp)
        ld s11, 200(sp)
        ld t3, 208(sp)
        ld t4, 216(sp)
        ld t5, 224(sp)
        ld t6, 232(sp)

        addi sp, sp, 240
        csrrw sp, mscratch, sp
        mret
        ",
    );
}
