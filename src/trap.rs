pub mod machine;
pub mod supervisor;

use core::arch::asm;
use riscv::register::mcause;
use riscv::register::mcause::{Exception, Interrupt};
use riscv::register::{mepc, mhartid, mip, mstatus, mtval, scause, sepc, stval, stvec};

/// CLINT MTIMECMP address
const MTIMECMP_ADDR: usize = 0x200_4000;

/// Trap handler for exception
unsafe fn trap_exception(a0: u64, a7: u64, exception_cause: Exception) {
    let ret_with_value = |ret_value: u64| {
        asm!("
            ld ra, 0(sp)
            ld t0, 8(sp)
            ld t1, 16(sp)
            ld t2, 24(sp)
            ld t3, 32(sp)
            ld t4, 40(sp)
            ld t5, 48(sp)
            ld t6, 56(sp)
            mv a0, {ret_value}
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
            ret_value = in(reg) ret_value,
        );
    };

    match exception_cause {
        // https://doxygen.coreboot.org/d6/dfc/sbi_8c_source.html
        Exception::UserEnvCall => {
            mepc::write(mepc::read() + 4);

            // ecall_number = a7
            let ecall_number: i64 = a7 as i64;
            match ecall_number {
                // sbi_set_timer
                0 => {
                    // timer_value = a0
                    let timer_value: u64 = a0;

                    let mtimecmp_addr = (MTIMECMP_ADDR + mhartid::read() * 8) as *mut u64;
                    mtimecmp_addr.write_volatile(timer_value);

                    ret_with_value(0);
                    unreachable!();
                }
                // sbi_clear_ipi
                3 => {
                    mip::clear_ssoft();

                    ret_with_value(0);
                    unreachable!();
                }
                // sbi_send_ipi
                4 => {
                    // mask_addr = a0
                    let mask_addr: *mut u64 = a0 as *mut u64;
                    let mut mask = if mstatus::read().mprv() {
                        mask_addr.read_volatile()
                    } else {
                        mstatus::set_mprv();
                        let mask = mask_addr.read_volatile();
                        mstatus::clear_mprv();
                        mask
                    };

                    let mut clint_addr: *mut u8 = 0x200_0000 as *mut u8;
                    while mask != 0 {
                        if mask & 1 == 1 {
                            clint_addr.write_volatile(1);
                        }
                        clint_addr = clint_addr.add(4);
                        mask >>= 1;
                    }

                    ret_with_value(0);
                    unreachable!();
                }
                // sbi_shutdown
                8 => panic!("sbi shutdown"),
                // other
                _ => panic!("unknown ecall number"),
            }
        }
        // other exception
        _ => {
            sepc::write(mepc::read());
            scause::write(mcause::read().bits());
            stval::write(mtval::read());
            mepc::write(stvec::read().bits() & !0x3);

            if mstatus::read().sie() {
                mstatus::set_spie();
            } else {
                // clear?
            }

            if mstatus::read().mpp() == mstatus::MPP::Supervisor {
                mstatus::set_spp(mstatus::SPP::Supervisor);
            } else {
                mstatus::set_spp(mstatus::SPP::User);
            }

            mstatus::clear_sie();
            mstatus::set_mpp(mstatus::MPP::Supervisor);
        }
    }
}

/// Trap handler for Interrupt
unsafe fn trap_interrupt(interrupt_cause: Interrupt) {
    match interrupt_cause {
        Interrupt::MachineSoft => {
            mip::set_ssoft();
            const CLINT_ADDR: usize = 0x200_0000;
            let interrupt_addr = (CLINT_ADDR + mhartid::read() * 4) as *mut u64;
            interrupt_addr.write_volatile(0);
        }
        Interrupt::MachineTimer => {
            mip::set_stimer();
            let mtimecmp_addr = (MTIMECMP_ADDR + mhartid::read() * 8) as *mut u64;
            mtimecmp_addr.write_volatile(u64::MAX);
        }
        Interrupt::MachineExternal => riscv::asm::wfi(), // wait for interrupt
        _ => panic!("unknown interrupt type"),
    }
}
