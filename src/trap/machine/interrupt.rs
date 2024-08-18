use super::mtrap_exit;
use crate::memmap::constant::device::{CLINT_ADDR, MTIMECMP_ADDR};
use riscv::register::mcause::Interrupt;
use riscv::register::{mhartid, mip};

/// Trap handler for Interrupt
pub unsafe fn trap_interrupt(interrupt_cause: Interrupt) -> ! {
    match interrupt_cause {
        Interrupt::MachineSoft => {
            mip::set_ssoft();
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

    mtrap_exit();
}
