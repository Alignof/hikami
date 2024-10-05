//! Emulation Zicfiss (Shadow Stack)
//! Ref: [https://github.com/riscv/riscv-cfi/releases/download/v1.0/riscv-cfi.pdf](https://github.com/riscv/riscv-cfi/releases/download/v1.0/riscv-cfi.pdf)

use super::pseudo_vs_exception;
use crate::memmap::page_table::constants::PAGE_SIZE;
use crate::memmap::HostPhysicalAddress;
use crate::PageBlock;
use crate::HYPERVISOR_DATA;

use core::cell::OnceCell;
use raki::{Instruction, OpcodeKind, ZicfissOpcode};
use spin::Mutex;

/// Singleton for Zicfiss.
/// TODO: change `OnceCell` to `LazyCell`.
pub static mut ZICFISS_DATA: Mutex<OnceCell<Zicfiss>> = Mutex::new(OnceCell::new());

/// Software-check exception. (cause value)
const SOFTWARE_CHECK_EXCEPTION: usize = 18;
/// Shadow stack fault. (tval value)
const SHADOW_STACK_FAULT: usize = 3;

/// Shadow Stack
struct ShadowStack {
    top: *const usize,
    bottom: *const usize,
    stack_pointer: *mut usize,
}

impl ShadowStack {
    /// Allocate memory region for shadow stack.
    pub fn new() -> Self {
        let stack_addr = PageBlock::alloc();
        let base_ptr = stack_addr.0 as *const usize;
        ShadowStack {
            top: unsafe { base_ptr.byte_add(PAGE_SIZE) },
            bottom: base_ptr,
            stack_pointer: unsafe { base_ptr.cast_mut().byte_add(PAGE_SIZE) },
        }
    }

    /// Push value to shadow stack
    pub fn push(&mut self, value: usize) {
        unsafe {
            self.stack_pointer = self.stack_pointer.byte_sub(core::mem::size_of::<usize>());
            if self.stack_pointer.cast_const() < self.bottom {
                panic!("stack smashed!");
            }
            self.stack_pointer.write_volatile(value);
        }
    }

    /// Pop value from shadow stack
    pub fn pop(&mut self) -> usize {
        unsafe {
            let pop_value = self.stack_pointer.read_volatile();
            self.stack_pointer = self.stack_pointer.byte_add(core::mem::size_of::<usize>());
            if self.stack_pointer.cast_const() > self.top {
                panic!("stack smashed!");
            }

            pop_value
        }
    }

    pub fn get_ssp(&self) -> HostPhysicalAddress {
        HostPhysicalAddress(self.stack_pointer as usize)
    }
}

/// Singleton for Zicfiss extension
pub struct Zicfiss {
    /// Shadow stack
    pub shadow_stack: ShadowStack,
    /// Shadow Stack Enable
    ///
    /// TODO: handle xenvcfg register.
    /// TODO: devide into each priv.
    pub sse: bool,
}

impl Zicfiss {
    pub fn new() -> Self {
        Zicfiss {
            shadow_stack: ShadowStack::new(),
            sse: false,
        }
    }
}

/// Emulate Zicfiss instruction.
pub fn instruction(inst: Instruction) {
    let mut context = unsafe { HYPERVISOR_DATA.lock().get().unwrap().guest().context };
    unsafe { ZICFISS_DATA.lock().get_or_init(|| Zicfiss::new()) };
    let mut zicfiss_data = unsafe { ZICFISS_DATA.lock() };
    let zicfiss = zicfiss_data.get_mut().unwrap();

    match inst.opc {
        OpcodeKind::Zicfiss(ZicfissOpcode::SSPUSH) => {
            if zicfiss.sse {
                let push_value = context.xreg(inst.rs2.unwrap());
                zicfiss.shadow_stack.push(push_value as usize);
            }
        }
        OpcodeKind::Zicfiss(ZicfissOpcode::C_SSPUSH) => {
            if zicfiss.sse {
                let push_value = context.xreg(inst.rd.unwrap());
                zicfiss.shadow_stack.push(push_value as usize);
            }
        }
        OpcodeKind::Zicfiss(ZicfissOpcode::SSPOPCHK) => {
            if zicfiss.sse {
                let pop_value = zicfiss.shadow_stack.pop();
                let expected_value = context.xreg(inst.rs1.unwrap()) as usize;
                if pop_value != expected_value {
                    pseudo_vs_exception(SOFTWARE_CHECK_EXCEPTION, SHADOW_STACK_FAULT)
                }
            }
        }
        OpcodeKind::Zicfiss(ZicfissOpcode::C_SSPOPCHK) => {
            if zicfiss.sse {
                let pop_value = zicfiss.shadow_stack.pop();
                let expected_value = context.xreg(inst.rd.unwrap()) as usize;
                if pop_value != expected_value {
                    pseudo_vs_exception(SOFTWARE_CHECK_EXCEPTION, SHADOW_STACK_FAULT)
                }
            }
        }
        OpcodeKind::Zicfiss(ZicfissOpcode::SSRDP) => {
            if zicfiss.sse {
                let ssp = zicfiss.shadow_stack.get_ssp();
                context.set_xreg(inst.rd.unwrap(), ssp.0 as u64);
            } else {
                context.set_xreg(inst.rd.unwrap(), 0);
            }
        }
        OpcodeKind::Zicfiss(ZicfissOpcode::SSAMOSWAP_W | ZicfissOpcode::SSAMOSWAP_D) => todo!(),
        _ => unreachable!(),
    }
}
