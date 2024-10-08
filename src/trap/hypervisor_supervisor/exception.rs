//! Trap VS-mode exception.

mod sbi_handler;

use super::hstrap_exit;
use crate::device::DeviceEmulateError;
use crate::guest;
use crate::h_extension::{
    csrs::{htinst, htval, vstvec},
    HvException,
};
use crate::memmap::HostPhysicalAddress;
use crate::HYPERVISOR_DATA;

use core::arch::asm;
use raki::{Decode, Instruction, Isa::Rv64, OpcodeKind, ZicntrOpcode};
use riscv::register::{
    scause::{self, Exception},
    stval,
};
use sbi_handler::{sbi_base_handler, sbi_rfnc_handler};

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
            sepc = in(reg) context.sepc(),
            scause = in(reg) scause::read().bits()
        );

        context.set_sepc(vstvec::read().bits());
    }
}

/// Handler for Ecall from VS-mode exception
#[allow(clippy::cast_possible_truncation)]
fn sbi_vs_mode_handler(context: &mut guest::context::Context) {
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
        sbi_spec::rfnc::EID_RFNC => sbi_rfnc_handler(func_id, arguments),
        _ => panic!(
            "Unsupported SBI call, eid: {:x}, fid: {:x}",
            ext_id, func_id
        ),
    };

    context.set_xreg(10, sbiret.error as u64);
    context.set_xreg(11, sbiret.value as u64);
}

/// Trap `VirtualInstruction` (cause = 22)
fn virtual_instruction_handler(inst_bytes: u32, context: &mut guest::context::Context) {
    let inst = inst_bytes
        .decode(Rv64)
        .expect("virtual instruction decoding failed");

    match inst.opc {
        OpcodeKind::Zicntr(ZicntrOpcode::RDTIME) => {
            let time_val = unsafe {
                let time;
                asm!("csrr {time_val}, time", time_val = out(reg) time);
                time
            };
            context.set_xreg(
                inst.rd.expect("rd register is not found in rdtime"),
                time_val,
            );
        }
        _ => panic!("unsupported instruction"),
    };
}

/// Trap handler for exception
#[allow(clippy::cast_possible_truncation, clippy::module_name_repetitions)]
pub unsafe fn trap_exception(exception_cause: Exception) -> ! {
    match exception_cause {
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
            HvException::LoadGuestPageFault => {
                let fault_addr = HostPhysicalAddress(htval::read().bits << 2);
                let fault_inst_value = htinst::read().bits;
                // htinst bit 1 replaced with a 0.
                // thus it needed to flip bit 1.
                // ref: vol. II p.161
                let fault_inst = Instruction::try_from(fault_inst_value | 0b10)
                    .expect("decoding load fault instruction failed");

                let mut hypervisor_data = HYPERVISOR_DATA.lock();
                match hypervisor_data
                    .get_mut()
                    .unwrap()
                    .devices()
                    .plic
                    .emulate_read(fault_addr)
                {
                    Ok(value) => {
                        let mut context = hypervisor_data.get().unwrap().guest().context;
                        context.set_xreg(fault_inst.rd.expect("rd is not found"), u64::from(value));
                        if (fault_inst_value & 0b10) >> 1 == 0 {
                            // compressed instruction
                            context.set_sepc(context.sepc() + 2);
                        } else {
                            // normal size instruction
                            context.set_sepc(context.sepc() + 4);
                        }
                    }
                    Err(
                        DeviceEmulateError::InvalidAddress
                        | DeviceEmulateError::InvalidContextId
                        | DeviceEmulateError::ReservedRegister,
                    ) => hs_forward_exception(),
                }
            }
            HvException::StoreAmoGuestPageFault => {
                let fault_addr = HostPhysicalAddress(htval::read().bits << 2);
                let fault_inst_value = htinst::read().bits;
                // htinst bit 1 replaced with a 0.
                // thus it needed to flip bit 1.
                // ref: vol. II p.161
                let fault_inst = Instruction::try_from(fault_inst_value | 0b10)
                    .expect("decoding load fault instruction failed");

                let mut hypervisor_data = HYPERVISOR_DATA.lock();
                let context = hypervisor_data.get().unwrap().guest().context;
                let update_epc = |fault_inst_value: usize, mut context: guest::context::Context| {
                    if (fault_inst_value & 0b10) >> 1 == 0 {
                        // compressed instruction
                        context.set_sepc(context.sepc() + 2);
                    } else {
                        // normal size instruction
                        context.set_sepc(context.sepc() + 4);
                    }
                };
                let store_value = context.xreg(fault_inst.rs2.expect("rs2 is not found"));

                if let Ok(()) = hypervisor_data
                    .get_mut()
                    .unwrap()
                    .devices()
                    .plic
                    .emulate_write(fault_addr, store_value.try_into().unwrap())
                {
                    update_epc(fault_inst_value, context);
                    drop(hypervisor_data);
                    hstrap_exit(); // exit handler
                }

                hs_forward_exception();
            }
            HvException::VirtualInstruction => {
                let mut context = unsafe { HYPERVISOR_DATA.lock().get().unwrap().guest().context };
                virtual_instruction_handler(stval::read() as u32, &mut context);
                context.set_sepc(context.sepc() + 4);
            }
        },
        _ => hs_forward_exception(),
    }

    hstrap_exit();
}
