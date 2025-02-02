//! Emulation Zbb

use super::{pseudo_vs_exception, EmulateExtension, EmulatedCsr};
use crate::HYPERVISOR_DATA;

use core::cell::OnceCell;
use raki::{Instruction, OpcodeKind, ZbbOpcode, ZicsrOpcode};
use spin::Mutex;

/// Singleton for Zbb.
pub static mut ZBB_DATA: Mutex<OnceCell<Zbb>> = Mutex::new(OnceCell::new());

/// Singleton for Zbb extension
pub struct Zbb;

impl Zbb {
    /// Constructor for `Zbb`.
    pub fn new() -> Self {
        Zbb
    }
}

impl EmulateExtension for Zbb {
    /// Emulate Zbb instruction.
    #[allow(clippy::cast_possible_truncation)]
    fn instruction(&mut self, inst: &Instruction) {
        const XLEN: usize = 64;
        let mut context = unsafe { HYPERVISOR_DATA.lock() }
            .get()
            .unwrap()
            .guest()
            .context;

        match inst.opc {
            OpcodeKind::Zbb(ZbbOpcode::RORIW) => todo!(),
            OpcodeKind::Zbb(ZbbOpcode::RORI) => {
                let input = context.xreg(inst.rs1.unwrap()) as usize;
                let shift_val = inst.imm.unwrap();
                let output = (input >> shift_val as usize) | (input << (XLEN - shift_val as usize));
                context.set_xreg(inst.rd.unwrap(), output as u64);
            }
            OpcodeKind::Zbb(ZbbOpcode::ROLW) => todo!(),
            OpcodeKind::Zbb(ZbbOpcode::RORW) => todo!(),
            OpcodeKind::Zbb(ZbbOpcode::ANDN) => {
                let rs1 = context.xreg(inst.rs1.unwrap());
                let rs2 = context.xreg(inst.rs2.unwrap());
                context.set_xreg(inst.rd.unwrap(), rs1 & !rs2);
            }
            OpcodeKind::Zbb(ZbbOpcode::ORN) => {
                let rs1 = context.xreg(inst.rs1.unwrap());
                let rs2 = context.xreg(inst.rs2.unwrap());
                context.set_xreg(inst.rd.unwrap(), rs1 | !rs2);
            }
            OpcodeKind::Zbb(ZbbOpcode::XNOR) => {
                let rs1 = context.xreg(inst.rs1.unwrap());
                let rs2 = context.xreg(inst.rs2.unwrap());
                context.set_xreg(inst.rd.unwrap(), rs1 ^ !rs2);
            }
            OpcodeKind::Zbb(ZbbOpcode::MAX) => todo!(),
            OpcodeKind::Zbb(ZbbOpcode::MAXU) => todo!(),
            OpcodeKind::Zbb(ZbbOpcode::MIN) => todo!(),
            OpcodeKind::Zbb(ZbbOpcode::MINU) => todo!(),
            OpcodeKind::Zbb(ZbbOpcode::ROL) => todo!(),
            OpcodeKind::Zbb(ZbbOpcode::ROR) => todo!(),
            OpcodeKind::Zbb(ZbbOpcode::SEXTB) => todo!(),
            OpcodeKind::Zbb(ZbbOpcode::SEXTH) => todo!(),
            OpcodeKind::Zbb(ZbbOpcode::ZEXTH) => todo!(),
            OpcodeKind::Zbb(ZbbOpcode::REV8) => todo!(),
            OpcodeKind::Zbb(ZbbOpcode::ORCB) => {
                const BYTE_SIZE: usize = 8;
                let input = context.xreg(inst.rs1.unwrap()) as usize;
                let mut output = 0;
                for start_bit in (0..XLEN).step_by(BYTE_SIZE) {
                    output |= if (input >> start_bit) & 0b1111_1111 == 0 {
                        0b0000_0000 << start_bit
                    } else {
                        0b1111_1111 << start_bit
                    }
                }
                context.set_xreg(inst.rd.unwrap(), output);
            }
            OpcodeKind::Zbb(ZbbOpcode::CPOP) => todo!(),
            OpcodeKind::Zbb(ZbbOpcode::CPOPW) => todo!(),
            OpcodeKind::Zbb(ZbbOpcode::CLZ) => {
                let rs1 = context.xreg(inst.rs1.unwrap());
                context.set_xreg(inst.rd.unwrap(), rs1.leading_zeros().into());
            }
            OpcodeKind::Zbb(ZbbOpcode::CLZW) => todo!(),
            OpcodeKind::Zbb(ZbbOpcode::CTZ) => todo!(),
            OpcodeKind::Zbb(ZbbOpcode::CTZW) => todo!(),
            _ => unreachable!(),
        }
    }

    /// Emulate Zicfiss CSRs access.
    fn csr(&mut self, inst: &Instruction) {
        todo!("Implementing Zbb CSR emulation");
    }

    /// Emulate CSR field that already exists.
    fn csr_field(&mut self, inst: &Instruction, write_to_csr_value: u64, read_csr_value: &mut u64) {
        todo!("Implementing Zbb CSR field emulation");
    }
}
