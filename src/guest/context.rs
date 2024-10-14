//! Guest context.

use crate::memmap::HostPhysicalAddress;

use raki::Instruction;

/// Guest context on memory
///
/// It place to hypervisor stack top.
#[repr(C)]
#[allow(dead_code)]
#[allow(clippy::module_name_repetitions)]
pub struct ContextData {
    /// Registers
    pub xreg: [u64; 32],
    /// Value of sstatus
    pub sstatus: usize,
    /// Program counter
    pub sepc: usize,
}

/// Guest context
#[derive(Debug, Copy, Clone)]
pub struct Context {
    address: HostPhysicalAddress,
}

impl Context {
    pub fn new(address: HostPhysicalAddress) -> Self {
        Context { address }
    }
}

impl Context {
    /// Get `ContextData` from raw address.
    #[allow(clippy::mut_from_ref)]
    fn get_context(&self) -> &mut ContextData {
        unsafe {
            (self.address.raw() as *mut ContextData)
                .as_mut()
                .expect("address of ContextData is invalid")
        }
    }

    /// Return regular register value.
    pub fn xreg(self, index: usize) -> u64 {
        self.get_context().xreg[index]
    }

    /// Set regular register value.
    pub fn set_xreg(&mut self, index: usize, value: u64) {
        self.get_context().xreg[index] = value;
    }

    /// Return sepc value.
    pub fn sepc(self) -> usize {
        self.get_context().sepc
    }

    /// Set sepc.
    pub fn set_sepc(&mut self, value: usize) {
        self.get_context().sepc = value;
    }

    /// Update sepc address according to instruction.
    pub fn update_sepc_by_inst(&mut self, inst: &Instruction) {
        if inst.is_compressed {
            // compressed instruction
            self.set_sepc(self.sepc() + 2);
        } else {
            // normal size instruction
            self.set_sepc(self.sepc() + 4);
        }
    }

    /// Return sstatus value.
    pub fn sstatus(self) -> usize {
        self.get_context().sstatus
    }

    /// Set sstatus.
    pub fn set_sstatus(&mut self, value: usize) {
        self.get_context().sstatus = value;
    }
}
