//! hikami library

#![no_std]

use raki::Instruction;

/// Trait for extention emulation.
pub trait EmulateExtension {
    /// Emulate instruction
    fn instruction(&mut self, inst: &Instruction);
    /// Emulate CSR
    fn csr(&mut self, inst: &Instruction);
    /// Emulate CSR field that already exists.
    fn csr_field(&mut self, inst: &Instruction, write_to_csr_value: u64, read_csr_value: &mut u64);
}

/// Holding a CSR value for CSRs emulation.
pub struct EmulatedCsr(u64);

impl EmulatedCsr {
    /// Create self
    pub fn new(value: u64) -> Self {
        EmulatedCsr(value)
    }

    /// Return raw data.
    pub fn bits(&self) -> u64 {
        self.0
    }

    /// Write data to CSR.
    /// For CSRRW or CSRRWI
    pub fn write(&mut self, data: u64) {
        self.0 = data;
    }

    /// Set bit in CSR.
    /// For CSRRS or CSRRSI
    pub fn set(&mut self, mask: u64) {
        self.0 |= mask;
    }

    /// Clear bit in CSR.
    /// For CSRRC or CSRRCI
    pub fn clear(&mut self, mask: u64) {
        self.0 &= !mask;
    }
}
