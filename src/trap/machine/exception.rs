use super::{mtrap_exit, mtrap_exit_sbi};
use crate::device::Device;
use crate::print;
use crate::{HYPERVISOR_DATA, SBI};
use riscv::register::{
    mcause::{self, Exception},
    mepc, mstatus, mtval, scause, sepc, stval, stvec,
};
use rustsbi::{RustSBI, Timer};
use sbi_spec::legacy;

/// Trap SBI Ecall
///
/// Handling SBI ecall is delegated to `Sbi` struct.
pub unsafe fn trap_envcall(a0: usize, a1: usize, a2: usize, a6: usize, a7: usize) -> ! {
    let sbi_cell = SBI.lock();
    let sbi_data = sbi_cell.get().unwrap();
    let ret_val = sbi_data.handle_ecall(a7, a6, [a0, a1, a2, 0, 0, 0]);

    mepc::write(mepc::read() + 4);

    if ret_val.error == 0 {
        drop(sbi_cell);
        mtrap_exit_sbi(ret_val.error, ret_val.value)
    } else {
        match a7 {
            // Set Timer (EID #0x00)
            legacy::LEGACY_SET_TIMER => {
                sbi_data.clint.set_timer(a0 as u64);
                drop(sbi_cell);
                mtrap_exit_sbi(0, 0)
            }
            // Console Putchar (EID #0x01)
            legacy::LEGACY_CONSOLE_PUTCHAR => {
                print!("{}", a0 as u8 as char);
                drop(sbi_cell);
                mtrap_exit_sbi(0, 0)
            }
            // Console Getchar (EID #0x02)
            legacy::LEGACY_CONSOLE_GETCHAR => {
                let uart_addr = sbi_data.uart.paddr() as *mut u32;
                let uart_lsr_addr = sbi_data.uart.lsr_addr() as *mut u32;

                while uart_lsr_addr.read_volatile() & 0x1 == 0 {}
                let c = uart_addr.read_volatile() as u8;
                drop(sbi_cell);
                mtrap_exit_sbi(0, c.into())
            }
            _ => panic!(
                "SBI call failed: error:{}, eid:{a7}, fid:{a6}",
                ret_val.error
            ),
        }
    }
}

/// Delegate exception to supervisor or user mode from machine mode.
#[no_mangle]
pub extern "C" fn forward_exception() {
    unsafe {
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

/// Trap handler for exception
#[allow(clippy::cast_possible_wrap)]
pub unsafe fn trap_exception(exception_cause: Exception) -> ! {
    match exception_cause {
        Exception::MachineEnvCall | Exception::SupervisorEnvCall | Exception::UserEnvCall => {
            let context = unsafe { HYPERVISOR_DATA.lock().guest().context };
            let a0 = context.xreg(10) as usize;
            let a1 = context.xreg(11) as usize;
            let a2 = context.xreg(12) as usize;
            let a6 = context.xreg(16) as usize;
            let a7 = context.xreg(17) as usize;
            trap_envcall(a0, a1, a2, a6, a7);
        }
        _ => {
            forward_exception();
            mtrap_exit();
        }
    }
}
