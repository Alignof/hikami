//! Handle instruction exceptions.
//!
//! - Illegal Instruction
//! - Virtual Instruction

use super::hs_forward_exception;
use hikami_core::HYPERVISOR_DATA;
use hikami_core::emulate_extension::EmulateExtension;

use raki::{Instruction, OpcodeKind};
use riscv::register::{sepc, stval};

extension_manager::import_global_variables!();

/// Trap `Illegal instruction` exception.
#[inline]
#[allow(clippy::similar_names)]
pub fn illegal_instruction() {
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
    extension_manager::handle_virtual_inst!();
}
