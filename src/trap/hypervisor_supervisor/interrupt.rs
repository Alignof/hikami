use crate::h_extension::csrs::vsip;
use crate::HYPERVISOR_DATA;
use riscv::register::scause::Interrupt;

/// Trap handler for Interrupt
pub unsafe fn trap_interrupt(interrupt_cause: Interrupt) {
    const CLINT_ADDR: usize = 0x200_0000;
    const MTIMECMP_ADDR: usize = 0x200_4000;

    let guest = &HYPERVISOR_DATA.lock().guest;

    match interrupt_cause {
        Interrupt::SupervisorSoft => {
            vsip::set_ssoft();
            let interrupt_addr = (CLINT_ADDR + guest.hart_id() * 4) as *mut u64;
            interrupt_addr.write_volatile(0);
        }
        Interrupt::SupervisorTimer => {
            vsip::set_stimer();
            let mtimecmp_addr = (MTIMECMP_ADDR + guest.hart_id() * 8) as *mut u64;
            mtimecmp_addr.write_volatile(u64::MAX);
        }
        Interrupt::SupervisorExternal => riscv::asm::wfi(), // wait for interrupt
        _ => panic!("unknown interrupt type"),
    }

    // restore context data
    guest.context.load();
}
