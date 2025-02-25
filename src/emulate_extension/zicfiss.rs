//! Emulation Zicfiss (Shadow Stack)
//! Ref: [https://github.com/riscv/riscv-cfi/releases/download/v1.0/riscv-cfi.pdf](https://github.com/riscv/riscv-cfi/releases/download/v1.0/riscv-cfi.pdf)

use super::pseudo_vs_exception;
use crate::memmap::{
    page_table::{g_stage_trans_addr, vs_stage_trans_addr},
    GuestVirtualAddress,
};
use crate::HYPERVISOR_DATA;
use hikami::{EmulateExtension, EmulatedCsr};

use core::cell::OnceCell;
use raki::{Instruction, OpcodeKind, ZicfissOpcode, ZicsrOpcode};
use spin::Mutex;

/// Singleton for Zicfiss.
/// TODO: change `OnceCell` to `LazyCell` when stable `LazyCell::force_mut`.
pub static mut ZICFISS_DATA: Mutex<OnceCell<Zicfiss>> = Mutex::new(OnceCell::new());

/// Software-check exception. (cause value)
const SOFTWARE_CHECK_EXCEPTION: usize = 18;
/// Store/AMO page fault
const STORE_AMO_PAGE_FAULT: usize = 15;
/// Shadow stack fault. (tval value)
const SHADOW_STACK_FAULT: usize = 3;

/// Singleton for Zicfiss extension
pub struct Zicfiss {
    /// Shadow stack pointer
    pub ssp: EmulatedCsr,
    /// Shadow Stack Enable in henvcfg (for VS-mode)
    pub henv_sse: bool,
    /// Shadow Stack Enable in senvcfg (for VU-mode)
    pub senv_sse: bool,
}

impl Zicfiss {
    /// Constructor for `Zicfiss`.
    pub fn new() -> Self {
        Zicfiss {
            ssp: EmulatedCsr::new(0),
            henv_sse: false,
            senv_sse: false,
        }
    }

    /// Return host physical shadow stack pointer as `*mut usize`.
    #[allow(clippy::similar_names, clippy::cast_possible_truncation)]
    fn ssp_hp_ptr(&self) -> *mut usize {
        if let Ok(gpa) = vs_stage_trans_addr(GuestVirtualAddress(self.ssp.bits() as usize)) {
            let hpa = g_stage_trans_addr(gpa).unwrap();
            hpa.0 as *mut usize
        } else {
            unsafe {
                HYPERVISOR_DATA.force_unlock();
                ZICFISS_DATA.force_unlock();
            }
            pseudo_vs_exception(STORE_AMO_PAGE_FAULT, self.ssp.bits() as usize);
        }
    }

    /// Push value to shadow stack
    pub fn ss_push(&mut self, value: usize) {
        unsafe {
            self.ssp = EmulatedCsr::new(
                (self.ssp.bits() as *const usize).byte_sub(core::mem::size_of::<usize>()) as u64,
            );
            self.ssp_hp_ptr().write_volatile(value);
        }
    }

    /// Pop value from shadow stack
    pub fn ss_pop(&mut self) -> usize {
        unsafe {
            let pop_value = self.ssp_hp_ptr().read_volatile();
            self.ssp = EmulatedCsr::new(
                (self.ssp.bits() as *const usize).byte_add(core::mem::size_of::<usize>()) as u64,
            );

            pop_value
        }
    }

    /// Is shadow stack enabled?
    ///
    /// Chack corresponding `SSE` bit of xenvcfg.
    fn is_ss_enable(&self, sstatus: usize) -> bool {
        let spp = (sstatus >> 8) & 0x1;
        if spp == 0 {
            self.senv_sse
        } else {
            self.henv_sse
        }
    }
}

impl EmulateExtension for Zicfiss {
    /// Emulate Zicfiss instruction.
    #[allow(clippy::cast_possible_truncation)]
    fn instruction(&mut self, inst: &Instruction) {
        let mut context = unsafe { HYPERVISOR_DATA.lock() }
            .get()
            .unwrap()
            .guest()
            .context;
        let sstatus = context.sstatus();

        match inst.opc {
            OpcodeKind::Zicfiss(ZicfissOpcode::SSPUSH) => {
                if self.is_ss_enable(sstatus) {
                    let push_value = context.xreg(inst.rs2.unwrap());
                    self.ss_push(push_value as usize);
                }
            }
            OpcodeKind::Zicfiss(ZicfissOpcode::C_SSPUSH) => {
                if self.is_ss_enable(sstatus) {
                    let push_value = context.xreg(inst.rd.unwrap());
                    self.ss_push(push_value as usize);
                }
            }
            OpcodeKind::Zicfiss(ZicfissOpcode::SSPOPCHK) => {
                if self.is_ss_enable(sstatus) {
                    let pop_value = self.ss_pop();
                    let expected_value = context.xreg(inst.rs1.unwrap()) as usize;
                    if pop_value != expected_value {
                        unsafe {
                            HYPERVISOR_DATA.force_unlock();
                            ZICFISS_DATA.force_unlock();
                        }
                        pseudo_vs_exception(SOFTWARE_CHECK_EXCEPTION, SHADOW_STACK_FAULT)
                    }
                }
            }
            OpcodeKind::Zicfiss(ZicfissOpcode::C_SSPOPCHK) => {
                if self.is_ss_enable(sstatus) {
                    let pop_value = self.ss_pop();
                    let expected_value = context.xreg(inst.rd.unwrap()) as usize;
                    if pop_value != expected_value {
                        unsafe {
                            HYPERVISOR_DATA.force_unlock();
                            ZICFISS_DATA.force_unlock();
                        }
                        pseudo_vs_exception(SOFTWARE_CHECK_EXCEPTION, SHADOW_STACK_FAULT)
                    }
                }
            }
            OpcodeKind::Zicfiss(ZicfissOpcode::SSRDP) => {
                if self.is_ss_enable(sstatus) {
                    context.set_xreg(inst.rd.unwrap(), self.ssp.bits());
                } else {
                    context.set_xreg(inst.rd.unwrap(), 0);
                }
            }
            OpcodeKind::Zicfiss(ZicfissOpcode::SSAMOSWAP_W | ZicfissOpcode::SSAMOSWAP_D) => todo!(),
            _ => todo!(),
        }
    }

    /// Emulate Zicfiss CSRs access.
    fn csr(&mut self, inst: &Instruction) {
        /// Register number of `Shadow Stack Pointer`.
        const CSR_SSP: usize = 0x11;

        let hypervisor_data = unsafe { HYPERVISOR_DATA.lock() };
        let mut context = hypervisor_data.get().unwrap().guest().context;

        let csr_num = inst.rs2.unwrap();
        match csr_num {
            CSR_SSP => match inst.opc {
                OpcodeKind::Zicsr(ZicsrOpcode::CSRRW) => {
                    let rs1 = context.xreg(inst.rs1.unwrap());
                    context.set_xreg(inst.rd.unwrap(), self.ssp.bits());
                    self.ssp.write(rs1);
                }
                OpcodeKind::Zicsr(ZicsrOpcode::CSRRS) => {
                    let rs1 = context.xreg(inst.rs1.unwrap());
                    context.set_xreg(inst.rd.unwrap(), self.ssp.bits());
                    self.ssp.set(rs1);
                }
                OpcodeKind::Zicsr(ZicsrOpcode::CSRRC) => {
                    let rs1 = context.xreg(inst.rs1.unwrap());
                    context.set_xreg(inst.rd.unwrap(), self.ssp.bits());
                    self.ssp.clear(rs1);
                }
                OpcodeKind::Zicsr(ZicsrOpcode::CSRRWI) => {
                    context.set_xreg(inst.rd.unwrap(), self.ssp.bits());
                    self.ssp.write(inst.rs1.unwrap() as u64);
                }
                OpcodeKind::Zicsr(ZicsrOpcode::CSRRSI) => {
                    context.set_xreg(inst.rd.unwrap(), self.ssp.bits());
                    self.ssp.set(inst.rs1.unwrap() as u64);
                }
                OpcodeKind::Zicsr(ZicsrOpcode::CSRRCI) => {
                    context.set_xreg(inst.rd.unwrap(), self.ssp.bits());
                    self.ssp.clear(inst.rs1.unwrap() as u64);
                }
                _ => unreachable!(),
            },
            unsupported_csr_num => {
                unimplemented!("unsupported CSRs: {unsupported_csr_num:#x}")
            }
        }
    }

    /// Emulate CSR field that already exists.
    fn csr_field(&mut self, inst: &Instruction, write_to_csr_value: u64, read_csr_value: &mut u64) {
        /// Register number of `Supervisor Environment Configuration Register`.
        const CSR_SENVCFG: usize = 0x10a;

        let csr_num = inst.rs2.unwrap();
        if csr_num == CSR_SENVCFG {
            // overwritten emulated csr field
            *read_csr_value |= u64::from(self.senv_sse) << 3;

            // update emulated csr field
            match inst.opc {
                OpcodeKind::Zicsr(
                    ZicsrOpcode::CSRRW
                    | ZicsrOpcode::CSRRS
                    | ZicsrOpcode::CSRRWI
                    | ZicsrOpcode::CSRRSI,
                ) => {
                    if (write_to_csr_value >> 3) & 0x1 == 1 {
                        self.senv_sse = true;
                    }
                }
                OpcodeKind::Zicsr(ZicsrOpcode::CSRRC | ZicsrOpcode::CSRRCI) => {
                    if (write_to_csr_value >> 3) & 0x1 == 1 {
                        self.senv_sse = false;
                    }
                }
                _ => unreachable!(),
            }
        }
    }
}
