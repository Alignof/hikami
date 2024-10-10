//! Extension emulation

pub mod zicfiss;

use crate::h_extension::csrs::vstvec;
use crate::trap::hypervisor_supervisor::hstrap_exit;
use crate::HYPERVISOR_DATA;

use core::arch::asm;
use raki::Instruction;
use riscv::register::sstatus;

/// Trait for extention emulation.
pub trait EmulateExtension {
    /// Emulate instruction
    fn instruction(&mut self, inst: Instruction);
    /// Emulate CSR
    fn csr(&mut self, inst: Instruction);
    /// Emulate CSR field that already exists.
    fn csr_field(&mut self, inst: &Instruction, write_to_csr_value: u64, read_csr_value: &mut u64);
}

/// CSR data for CSRs emulation.
pub struct CsrData(u64);

impl CsrData {
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

/// Throw an VS-level exception.
/// * `exception_num`: Exception number. (store to vscause)
/// * `trap_value`: Trap value. (store to vstval)
pub fn pseudo_vs_exception(exception_num: usize, trap_value: usize) -> ! {
    unsafe {
        let hypervisor_data = HYPERVISOR_DATA.lock();
        let mut context = hypervisor_data.get().unwrap().guest().context;
        asm!(
            "csrw vsepc, {sepc}",
            "csrw vscause, {cause}",
            "csrw vstval, {tval}",
            sepc = in(reg) context.sepc(),
            cause = in(reg) exception_num,
            tval = in(reg) trap_value,
        );

        let spp = sstatus::read().spp();
        let vsstatus: usize;
        asm!("csrr {status}, vsstatus", status = out(reg) vsstatus);
        let sie = vsstatus >> 1 & 0x1;
        asm!(
            "csrw vsstatus, {status}",
            status = in(reg) (vsstatus & !(1 << 8)) | (spp as usize) << 8
        );
        // disable interrupt
        asm!(
            "csrs vsstatus, {status}",
            "csrci vsstatus, 0b10",
            status = in(reg) sie << 5,
        );
        context.set_sstatus(context.sstatus() | 1 << 8);

        context.set_sepc(vstvec::read().bits());

        drop(hypervisor_data);

        hstrap_exit();
    }
}
