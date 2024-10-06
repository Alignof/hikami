//! Emulation Zicfiss (Shadow Stack)
//! Ref: [https://github.com/riscv/riscv-cfi/releases/download/v1.0/riscv-cfi.pdf](https://github.com/riscv/riscv-cfi/releases/download/v1.0/riscv-cfi.pdf)

use super::{pseudo_vs_exception, CsrData};
use crate::HYPERVISOR_DATA;

use core::cell::OnceCell;
use raki::{Instruction, OpcodeKind, ZicfissOpcode, ZicsrOpcode};
use spin::Mutex;

/// Singleton for Zicfiss.
/// TODO: change `OnceCell` to `LazyCell`.
pub static mut ZICFISS_DATA: Mutex<OnceCell<Zicfiss>> = Mutex::new(OnceCell::new());

/// Software-check exception. (cause value)
const SOFTWARE_CHECK_EXCEPTION: usize = 18;
/// Shadow stack fault. (tval value)
const SHADOW_STACK_FAULT: usize = 3;

/// Singleton for Zicfiss extension
pub struct Zicfiss {
    /// Shadow stack pointer
    pub ssp: CsrData,

    /// Shadow Stack Enable
    ///
    /// TODO: handle xenvcfg register.
    /// TODO: devide into each priv.
    pub sse: bool,
}

impl Zicfiss {
    pub fn new() -> Self {
        Zicfiss {
            ssp: CsrData(0),
            sse: false,
        }
    }

    fn ssp_ptr(&self) -> *mut usize {
        self.ssp.0 as *mut usize
    }

    /// Push value to shadow stack
    pub fn ss_push(&mut self, value: usize) {
        unsafe {
            self.ssp = CsrData(self.ssp_ptr().byte_sub(core::mem::size_of::<usize>()) as u64);
            self.ssp_ptr().write_volatile(value);
        }
    }

    /// Pop value from shadow stack
    pub fn ss_pop(&mut self) -> usize {
        unsafe {
            let pop_value = self.ssp_ptr().read_volatile();
            self.ssp = CsrData(self.ssp_ptr().byte_add(core::mem::size_of::<usize>()) as u64);

            pop_value
        }
    }
}

/// Emulate Zicfiss instruction.
pub fn instruction(inst: Instruction) {
    let hypervisor_data = unsafe { HYPERVISOR_DATA.lock() };
    let mut context = hypervisor_data.get().unwrap().guest().context;
    unsafe { ZICFISS_DATA.lock().get_or_init(|| Zicfiss::new()) };
    let mut zicfiss_data = unsafe { ZICFISS_DATA.lock() };
    let zicfiss = zicfiss_data.get_mut().unwrap();

    match inst.opc {
        OpcodeKind::Zicfiss(ZicfissOpcode::SSPUSH) => {
            if zicfiss.sse {
                let push_value = context.xreg(inst.rs2.unwrap());
                zicfiss.ss_push(push_value as usize);
            }
        }
        OpcodeKind::Zicfiss(ZicfissOpcode::C_SSPUSH) => {
            if zicfiss.sse {
                let push_value = context.xreg(inst.rd.unwrap());
                zicfiss.ss_push(push_value as usize);
            }
        }
        OpcodeKind::Zicfiss(ZicfissOpcode::SSPOPCHK) => {
            if zicfiss.sse {
                let pop_value = zicfiss.ss_pop();
                let expected_value = context.xreg(inst.rs1.unwrap()) as usize;
                if pop_value != expected_value {
                    drop(zicfiss_data);
                    drop(hypervisor_data);
                    pseudo_vs_exception(SOFTWARE_CHECK_EXCEPTION, SHADOW_STACK_FAULT)
                }
            }
        }
        OpcodeKind::Zicfiss(ZicfissOpcode::C_SSPOPCHK) => {
            if zicfiss.sse {
                let pop_value = zicfiss.ss_pop();
                let expected_value = context.xreg(inst.rd.unwrap()) as usize;
                if pop_value != expected_value {
                    drop(zicfiss_data);
                    drop(hypervisor_data);
                    pseudo_vs_exception(SOFTWARE_CHECK_EXCEPTION, SHADOW_STACK_FAULT)
                }
            }
        }
        OpcodeKind::Zicfiss(ZicfissOpcode::SSRDP) => {
            if zicfiss.sse {
                context.set_xreg(inst.rd.unwrap(), zicfiss.ssp.0 as u64);
            } else {
                context.set_xreg(inst.rd.unwrap(), 0);
            }
        }
        OpcodeKind::Zicfiss(ZicfissOpcode::SSAMOSWAP_W | ZicfissOpcode::SSAMOSWAP_D) => todo!(),
        _ => todo!(),
    }
}

/// Emulate Zicfiss CSRs access.
pub fn csrs(inst: Instruction) {
    const CSR_SSP: usize = 0x11;

    let hypervisor_data = unsafe { HYPERVISOR_DATA.lock() };
    let mut context = hypervisor_data.get().unwrap().guest().context;
    let mut zicfiss_data = unsafe { ZICFISS_DATA.lock() };
    let zicfiss = zicfiss_data.get_mut().unwrap();

    let csr_num = inst.rs2.unwrap();
    match csr_num {
        CSR_SSP => match inst.opc {
            OpcodeKind::Zicsr(ZicsrOpcode::CSRRW) => {
                let rs1 = context.xreg(inst.rs1.unwrap());
                context.set_xreg(inst.rd.unwrap(), zicfiss.ssp.bits());
                zicfiss.ssp.write(rs1);
            }
            OpcodeKind::Zicsr(ZicsrOpcode::CSRRS) => {
                let rs1 = context.xreg(inst.rs1.unwrap());
                context.set_xreg(inst.rd.unwrap(), zicfiss.ssp.bits());
                zicfiss.ssp.set(rs1);
            }
            OpcodeKind::Zicsr(ZicsrOpcode::CSRRC) => {
                let rs1 = context.xreg(inst.rs1.unwrap());
                context.set_xreg(inst.rd.unwrap(), zicfiss.ssp.bits());
                zicfiss.ssp.clear(rs1);
            }
            OpcodeKind::Zicsr(ZicsrOpcode::CSRRWI) => {
                context.set_xreg(inst.rd.unwrap(), zicfiss.ssp.bits());
                zicfiss.ssp.write(inst.rs1.unwrap() as u64);
            }
            OpcodeKind::Zicsr(ZicsrOpcode::CSRRSI) => {
                context.set_xreg(inst.rd.unwrap(), zicfiss.ssp.bits());
                zicfiss.ssp.set(inst.rs1.unwrap() as u64);
            }
            OpcodeKind::Zicsr(ZicsrOpcode::CSRRCI) => {
                context.set_xreg(inst.rd.unwrap(), zicfiss.ssp.bits());
                zicfiss.ssp.clear(inst.rs1.unwrap() as u64);
            }
            _ => unreachable!(),
        },
        unsupported_csr_num => {
            unimplemented!("unsupported CSRs: {unsupported_csr_num:#x}")
        }
    }
}
