use crate::memmap::GuestPhysicalAddress;

/// Guest context on memory
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
    address: GuestPhysicalAddress,
}

impl Context {
    pub fn new(address: GuestPhysicalAddress) -> Self {
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

    pub fn xreg(self, index: usize) -> u64 {
        self.get_context().xreg[index]
    }

    pub fn set_xreg(&mut self, index: usize, value: u64) {
        self.get_context().xreg[index] = value;
    }

    pub fn sepc(self) -> usize {
        self.get_context().sepc
    }

    pub fn set_sepc(&mut self, value: usize) {
        self.get_context().sepc = value;
    }

    pub fn set_sstatus(&mut self, value: usize) {
        self.get_context().sstatus = value;
    }
}
