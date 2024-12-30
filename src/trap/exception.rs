//! Trap VS-mode exception.

mod instruction_handler;
mod page_fault_handler;
mod sbi_handler;

use super::hstrap_exit;
use crate::guest;
use crate::h_extension::{csrs::vstvec, HvException};
use crate::HYPERVISOR_DATA;
use sbi_handler::sbi_call;

use core::arch::asm;
use riscv::register::{
    scause::{self, Exception},
    stval,
};
use sbi_handler::{sbi_base_handler, sbi_fwft_handler, sbi_pmu_handler, sbi_rfnc_handler};

/// Delegate exception to supervisor mode from VS-mode.
#[no_mangle]
#[inline(always)]
#[allow(clippy::inline_always, clippy::module_name_repetitions)]
pub extern "C" fn hs_forward_exception() {
    unsafe {
        let mut context = HYPERVISOR_DATA.lock().get().unwrap().guest().context;
        asm!(
            "csrw vsepc, {sepc}",
            "csrw vscause, {scause}",
            "csrw vstval, {stval}",
            sepc = in(reg) context.sepc(),
            scause = in(reg) scause::read().bits(),
            stval = in(reg) stval::read(),
        );

        context.set_sepc(vstvec::read().bits());
    }
}

/// Handler for Ecall from VS-mode exception
#[allow(clippy::cast_possible_truncation)]
fn sbi_vs_mode_handler(context: &mut guest::context::Context) {
    /// Extension ID of FWFT(Firmware Features) Extension.
    const EID_FWFT: usize = 0x4657_4654;

    let ext_id: usize = context.xreg(17) as usize;
    let func_id: usize = context.xreg(16) as usize;
    let arguments: &[u64; 5] = &[
        context.xreg(10),
        context.xreg(11),
        context.xreg(12),
        context.xreg(13),
        context.xreg(14),
    ];

    let sbiret = match ext_id {
        sbi_spec::base::EID_BASE => sbi_base_handler(func_id),
        sbi_spec::pmu::EID_PMU => sbi_pmu_handler(func_id, arguments),
        sbi_spec::rfnc::EID_RFNC => sbi_rfnc_handler(func_id, arguments),
        EID_FWFT => sbi_fwft_handler(func_id, arguments),
        _ => sbi_call(ext_id, func_id, arguments),
    };

    context.set_xreg(10, sbiret.error as u64);
    context.set_xreg(11, sbiret.value as u64);
}

/// Update sepc by htinst value.
fn update_sepc_by_htinst_value(htinst_inst_value: usize, context: &mut guest::context::Context) {
    if (htinst_inst_value & 0b10) >> 1 == 0 {
        // compressed instruction
        context.set_sepc(context.sepc() + 2);
    } else {
        // normal size instruction
        context.set_sepc(context.sepc() + 4);
    }
}

/// Trap handler for exception
#[allow(clippy::cast_possible_truncation, clippy::module_name_repetitions)]
pub unsafe fn trap_exception(exception_cause: Exception) -> ! {
    match exception_cause {
        Exception::IllegalInstruction => instruction_handler::illegal_instruction(),
        Exception::SupervisorEnvCall => panic!("SupervisorEnvCall should be handled by M-mode"),
        // Enum not found in `riscv` crate.
        Exception::Unknown => match HvException::from(scause::read().code()) {
            HvException::EcallFromVsMode => {
                let mut context = unsafe { HYPERVISOR_DATA.lock().get().unwrap().guest().context };
                sbi_vs_mode_handler(&mut context);
                context.set_sepc(context.sepc() + 4);
            }
            HvException::InstructionGuestPageFault => {
                panic!("Instruction guest-page fault");
            }
            HvException::LoadGuestPageFault => page_fault_handler::load_guest_page_fault(),
            HvException::StoreAmoGuestPageFault => page_fault_handler::store_guest_page_fault(),
            HvException::VirtualInstruction => instruction_handler::virtual_instruction(),
        },
        _ => hs_forward_exception(),
    }

    hstrap_exit();
}
