use riscv::register::scause::Interrupt;
use riscv::register::{mhartid, mip};

/// Trap handler for Interrupt
pub unsafe fn trap_interrupt(interrupt_cause: Interrupt) {
    const CLINT_ADDR: usize = 0x200_0000;
    const MTIMECMP_ADDR: usize = 0x200_4000;

    match interrupt_cause {
        Interrupt::SupervisorSoft => {
            mip::set_ssoft();
            let interrupt_addr = (CLINT_ADDR + mhartid::read() * 4) as *mut u64;
            interrupt_addr.write_volatile(0);
        }
        Interrupt::SupervisorTimer => {
            mip::set_stimer();
            let mtimecmp_addr = (MTIMECMP_ADDR + mhartid::read() * 8) as *mut u64;
            mtimecmp_addr.write_volatile(u64::MAX);
        }
        Interrupt::SupervisorExternal => riscv::asm::wfi(), // wait for interrupt
        _ => panic!("unknown interrupt type"),
    }
}
