//! Handle instruction exceptions.

use crate::emulate_extension::zicfiss::ZICFISS_DATA;
use crate::emulate_extension::EmulateExtension;
use crate::HYPERVISOR_DATA;

use core::arch::asm;
use raki::{Instruction, OpcodeKind};
use riscv::register::{sepc, stval};

/// Trap `Illegal instruction` exception.
#[inline]
pub fn illegal_instruction() {
    let fault_inst_value = stval::read();
    let fault_inst =
        Instruction::try_from(fault_inst_value).expect("decoding load fault instruction failed");

    // emulate the instruction
    match fault_inst.opc {
        OpcodeKind::Zicfiss(_) => unsafe { ZICFISS_DATA.lock() }
            .get_mut()
            .unwrap()
            .instruction(&fault_inst),
        OpcodeKind::Zicsr(_) => match fault_inst.rs2.unwrap() {
            // ssp
            0x11 => unsafe { ZICFISS_DATA.lock() }
                .get_mut()
                .unwrap()
                .csr(&fault_inst),
            unsupported_csr_num => {
                unimplemented!("unsupported CSRs: {unsupported_csr_num:#x}")
            }
        },
        _ => unimplemented!(
            "unsupported illegal instruction: {:#?}, at {:#x}",
            fault_inst,
            sepc::read()
        ),
    }

    let mut context = unsafe { HYPERVISOR_DATA.lock().get().unwrap().guest().context };
    context.update_sepc_by_inst(&fault_inst);
}

/// Trap `Virtual instruction` exception.
#[inline]
pub fn virtual_instruction() {
    let fault_inst_value = stval::read();
    let fault_inst =
        Instruction::try_from(fault_inst_value).expect("decoding load fault instruction failed");
    let mut context = unsafe { HYPERVISOR_DATA.lock() }
        .get()
        .unwrap()
        .guest()
        .context;

    // emulate CSR set
    match fault_inst.opc {
        OpcodeKind::Zicsr(_) => {
            match fault_inst.rs2.unwrap() {
                // senvcfg
                0x10a => {
                    let mut read_from_csr_value: u64;
                    unsafe {
                        asm!("csrr {0}, senvcfg", out(reg) read_from_csr_value);
                    }

                    let write_to_csr_value = context.xreg(fault_inst.rs1.unwrap());

                    // update emulated CSR field.
                    unsafe { ZICFISS_DATA.lock() }.get_mut().unwrap().csr_field(
                        &fault_inst,
                        write_to_csr_value,
                        &mut read_from_csr_value,
                    );

                    // commit result
                    unsafe {
                        asm!("csrw senvcfg, {0}", in(reg) write_to_csr_value);
                    }
                    context.set_xreg(fault_inst.rd.unwrap(), read_from_csr_value);
                }
                unsupported_csr_num => {
                    unimplemented!("unsupported CSRs: {unsupported_csr_num:#x}")
                }
            }
        }
        _ => unreachable!(),
    }

    context.update_sepc_by_inst(&fault_inst);
}
