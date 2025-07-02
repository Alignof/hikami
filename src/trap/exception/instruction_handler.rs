//! Handle instruction exceptions.
//!
//! - Illegal Instruction
//! - Virtual Instruction

use super::hs_forward_exception;
use hikami_core::emulate_extension::EmulateExtension;
use hikami_core::HYPERVISOR_DATA;

use raki::{Instruction, OpcodeKind};
use riscv::register::{sepc, stval};

extension_manager::import_global_variables!();

/// Trap `Illegal instruction` exception.
#[inline]
pub fn illegal_instruction() {
    let fault_inst_value = stval::read();
    let fault_inst = Instruction::try_from(fault_inst_value).unwrap_or_else(|_| {
        panic!("decoding load fault instruction failed: fault inst value: {fault_inst_value:#x} at {:#x}", sepc::read());
    });

    // emulate the instruction
    extension_manager::handle_illegal_inst!();
}

/// Trap `Virtual instruction` exception.
#[inline]
pub fn virtual_instruction() {
    let fault_inst_value = stval::read();
    let fault_inst = Instruction::try_from(fault_inst_value).unwrap_or_else(|_| {
        panic!("decoding load fault instruction failed: fault inst value: {fault_inst_value:#x} at {:#x}", sepc::read());
    });

    // emulate CSR set
    extension_manager::handle_virtual_inst!();
}
