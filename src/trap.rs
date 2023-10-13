use crate::memmap::constant::STACK_BASE;
use core::arch::asm;
use riscv::register::mcause;
use riscv::register::mcause::Trap;
use riscv::register::mcause::{Exception, Interrupt};
use riscv::register::{mepc, mhartid, mip, mstatus, mtval, scause, sepc, stval, stvec};

#[no_mangle]
unsafe fn mtrap() {
    asm!(
        ".align 4
        csrrw mscratch, sp
        li sp, {stack_base}

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

    const MTIMECMP_ADDR: usize = 0x200_4000;
    let trap_cause = mcause::read();
    match trap_cause.cause() {
        Trap::Interrupt(inter) => match inter {
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
            Interrupt::MachineExternal => loop {},
            _ => panic!("unknown interrupt type"),
        },
        Trap::Exception(except) => match except {
            // https://doxygen.coreboot.org/d6/dfc/sbi_8c_source.html
            Exception::UserEnvCall => {
                mepc::write(mepc::read() + 4);

                // ecall_number = a7
                let ecall_number: i64 = 0;
                asm!("ld {ecall_number}, 120(sp)", ecall_number = in(reg) ecall_number);
                match ecall_number {
                    // sbi_set_timer
                    0 => {
                        // timer_value = a0
                        let timer_value: u64 = 0;
                        asm!("ld {timer_value}, 64(sp)", timer_value = in(reg) timer_value);

                        let mtimecmp_addr = (MTIMECMP_ADDR + mhartid::read() * 8) as *mut u64;
                        mtimecmp_addr.write_volatile(timer_value);
                        asm!("li a0, 0");
                    }
                    // sbi_clear_ipi
                    3 => {
                        mip::clear_ssoft();
                        asm!("li a0, 0");
                    }
                    // sbi_send_ipi
                    4 => {
                        // mask_addr = a0
                        let mask_addr: *mut u64 = core::ptr::null_mut();
                        asm!("ld {mask_addr}, 64(sp)", mask_addr = in(reg) mask_addr);

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
                        asm!("li a0, 0");
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
        },
    }

    asm!("
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

        csrrw mscratch, sp
        li sp, {stack_base}

        mret
        ",
        stack_base = in(reg) STACK_BASE
    );
}
