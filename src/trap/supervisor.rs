use super::{trap_exception, trap_interrupt};
use crate::memmap::constant::STACK_BASE;
use core::arch::asm;
use riscv::register::mcause;
use riscv::register::mcause::Trap;

#[no_mangle]
pub unsafe fn strap_vector() {
    asm!(
        ".align 4
        csrrw sp, sscratch, sp
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
    asm!("ld {a7_reg}, 64(sp)", a7_reg = in(reg) a7);
    match mcause::read().cause() {
        Trap::Interrupt(interrupt_cause) => trap_interrupt(interrupt_cause),
        Trap::Exception(exception_cause) => trap_exception(a0, a7, exception_cause),
    }

    asm!(
        "
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

        addi sp, sp, 240
        csrrw sp, sscratch, sp
        sret
        ",
    );
}
