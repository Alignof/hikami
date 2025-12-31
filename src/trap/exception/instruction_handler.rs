//! Handle instruction exceptions.
//!
//! - Illegal Instruction
//! - Virtual Instruction

use super::hs_forward_exception;
use hikami_core::HYPERVISOR_DATA;
use hikami_core::emulate_extension::EmulateExtension;

use core::arch::asm;
use raki::{Instruction, OpcodeKind, ZicsrOpcode};
use riscv::register::{sepc, stval};

extension_manager::import_global_variables!();

/// Decode instruction which caused the exception.
#[inline]
fn decode_instruction(fault_inst_value: usize) -> Instruction {
    let mut hypervisor = unsafe { HYPERVISOR_DATA.lock() };
    let hypervisor = hypervisor.get_mut().unwrap();
    if let Some(cached_inst) = hypervisor
        .guest()
        .instruction_cache
        .get(&fault_inst_value)
        .copied()
    {
        return cached_inst;
    }

    let inst = Instruction::try_from(fault_inst_value).unwrap_or_else(|_| {
            use hikami_core::memmap::GuestVirtualAddress;
            let gva = GuestVirtualAddress(sepc::read());
            let gpa = hikami_core::memmap::page_table::vs_stage_trans_addr(gva).unwrap();
            let hpa = hikami_core::memmap::page_table::g_stage_trans_addr(gpa).unwrap();

            panic!(
                "decoding load fault instruction failed: fault inst value: {fault_inst_value:#x} at {:#x}(GPA: {:#x}, HPA: {:#x})",
                sepc::read(), gpa.raw(), hpa.raw()
            );
        });

    hypervisor
        .guest_mut()
        .instruction_cache
        .insert(fault_inst_value, inst);

    inst
}

/// Trap `Illegal instruction` exception.
#[inline]
#[allow(clippy::similar_names)]
pub fn illegal_instruction() {
    let fault_inst_value = stval::read();
    let fault_inst = decode_instruction(fault_inst_value);

    // emulate the instruction
    extension_manager::handle_illegal_inst!();
}

/// Trap `Virtual instruction` exception.
#[inline]
#[allow(clippy::similar_names)]
pub fn virtual_instruction() {
    let fault_inst_value = stval::read();
    let fault_inst = Instruction::try_from(fault_inst_value).unwrap_or_else(|_| {
        use hikami_core::memmap::GuestVirtualAddress;
        let gva = GuestVirtualAddress(sepc::read());
        let gpa = hikami_core::memmap::page_table::vs_stage_trans_addr(gva).unwrap();
        let hpa = hikami_core::memmap::page_table::g_stage_trans_addr(gpa).unwrap();

        panic!(
            "decoding load fault instruction failed: fault inst value: {fault_inst_value:#x} at {:#x}(GPA: {:#x}, HPA: {:#x})",
            sepc::read(), gpa.raw(), hpa.raw()
        );
    });

    // emulate CSR set
    if fault_inst.rs2.unwrap() == 0x10a {
        let mut context = unsafe { HYPERVISOR_DATA.lock() }
            .get()
            .unwrap()
            .guest()
            .context;

        let mut csr: u64;
        unsafe {
            asm!("csrr {0}, senvcfg", out(reg) csr);
        }

        #[allow(clippy::cast_sign_loss)]
        let new_csr = match fault_inst.opc {
            OpcodeKind::Zicsr(ZicsrOpcode::CSRRW) => context.xreg(fault_inst.rs1.unwrap()),
            OpcodeKind::Zicsr(ZicsrOpcode::CSRRS) => {
                let rs1 = context.xreg(fault_inst.rs1.unwrap());
                csr | rs1
            }
            OpcodeKind::Zicsr(ZicsrOpcode::CSRRC) => {
                let rs1 = context.xreg(fault_inst.rs1.unwrap());
                csr & !rs1
            }
            OpcodeKind::Zicsr(ZicsrOpcode::CSRRWI) => fault_inst.imm.unwrap() as u64,
            OpcodeKind::Zicsr(ZicsrOpcode::CSRRSI) => {
                let imm = fault_inst.imm.unwrap() as u64;
                csr | imm
            }
            OpcodeKind::Zicsr(ZicsrOpcode::CSRRCI) => {
                let imm = fault_inst.imm.unwrap() as u64;
                csr & !imm
            }
            _ => unreachable!(),
        };

        // commit result
        unsafe {
            asm!("csrw senvcfg, {0}", in(reg) new_csr);
        }
        context.set_xreg(fault_inst.rd.unwrap(), csr);

        context.update_sepc_by_inst(&fault_inst);

        return;
    }

    // emulate CSR set
    extension_manager::handle_virtual_inst!();
}
