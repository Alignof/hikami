//! PLIC: Platform-Level Interrupt Controller  
//! ref: [https://github.com/riscv/riscv-plic-spec/releases/download/1.0.0/riscv-plic-1.0.0.pdf](https://github.com/riscv/riscv-plic-spec/releases/download/1.0.0/riscv-plic-1.0.0.pdf)

use super::{Device, PTE_FLAGS_FOR_DEVICE};
use crate::h_extension::csrs::{hvip, VsInterruptKind};
use crate::memmap::constant::MAX_HART_NUM;
use crate::memmap::{GuestPhysicalAddress, HostPhysicalAddress, MemoryMap};
use fdt::Fdt;

/// Max number of PLIC context.
pub const MAX_CONTEXT_NUM: usize = MAX_HART_NUM * 2;

// unused constant for now
// pub const ENABLE_BASE: usize = 0x2000;
// pub const ENABLE_PER_HART: usize = 0x80;
const CONTEXT_BASE: usize = 0x20_0000;
const CONTEXT_REGS_SIZE: usize = 0x1000;
const CONTEXT_CLAIM: usize = 0x4;

/// PLIC emulation result.
pub enum PlicEmulateError {
    InvalidAddress,
}

/// PLIC: Platform-Level Interrupt Controller  
/// Interrupt controller for global interrupts.
#[derive(Debug)]
pub struct Plic {
    base_addr: HostPhysicalAddress,
    size: usize,
    claim_complete: [u32; MAX_CONTEXT_NUM],
}

impl Plic {
    /// Read plic claim/update register and reflect to `claim_complete`.
    pub fn update_claim_complete(&mut self, hart_id: usize) {
        let claim_complete_addr =
            self.base_addr + CONTEXT_BASE + CONTEXT_REGS_SIZE * hart_id + CONTEXT_CLAIM;
        let irq = unsafe { core::ptr::read_volatile(claim_complete_addr.raw() as *const u32) };
        self.claim_complete[hart_id] = irq;
    }

    /// Emulate reading plic register.
    pub fn emulate_read(&self, dst_addr: HostPhysicalAddress) -> Result<usize, PlicEmulateError> {
        let offset = dst_addr.raw() - self.base_addr.raw();
        if offset < CONTEXT_BASE || offset > CONTEXT_BASE + CONTEXT_REGS_SIZE * MAX_CONTEXT_NUM {
            return Err(PlicEmulateError::InvalidAddress);
        }

        let context_id = (offset - CONTEXT_BASE) / CONTEXT_REGS_SIZE;
        Ok(self.claim_complete[context_id] as usize)
    }

    /// Emulate writing plic register.
    pub fn emulate_write(
        &mut self,
        dst_addr: HostPhysicalAddress,
        value: u32,
    ) -> Result<(), PlicEmulateError> {
        let offset = dst_addr.raw() - self.base_addr.raw();
        if offset < CONTEXT_BASE || offset > CONTEXT_BASE + CONTEXT_REGS_SIZE * MAX_CONTEXT_NUM {
            return Err(PlicEmulateError::InvalidAddress);
        }
        let offset_per_context = offset % CONTEXT_REGS_SIZE;

        let context_id = (offset - CONTEXT_BASE) / CONTEXT_REGS_SIZE;
        match offset_per_context {
            // threshold
            0 => {
                let dst_ptr = dst_addr.raw() as *mut u32;
                unsafe {
                    dst_ptr.write_volatile(value);
                }
            }
            // claim/complete
            4 => {
                let dst_ptr = dst_addr.raw() as *mut u32;
                unsafe {
                    dst_ptr.write_volatile(value);
                }
                self.claim_complete[context_id] = 0;
                hvip::clear(VsInterruptKind::External);
            }
            8 => panic!("offset 8 is reserved"),
            _ => return Err(PlicEmulateError::InvalidAddress),
        }

        Ok(())
    }
}

impl Device for Plic {
    fn new(device_tree: &Fdt, node_path: &str) -> Self {
        let region = device_tree
            .find_node(node_path)
            .unwrap()
            .reg()
            .unwrap()
            .next()
            .unwrap();

        Plic {
            base_addr: HostPhysicalAddress(region.starting_address as usize),
            size: region.size.unwrap(),
            claim_complete: [0u32; MAX_HART_NUM],
        }
    }

    fn size(&self) -> usize {
        self.size
    }

    fn paddr(&self) -> HostPhysicalAddress {
        self.base_addr
    }

    fn memmap(&self) -> MemoryMap {
        let vaddr = GuestPhysicalAddress(self.paddr().raw());
        MemoryMap::new(
            vaddr..vaddr + self.size(),
            self.paddr()..self.paddr() + self.size(),
            &PTE_FLAGS_FOR_DEVICE,
        )
    }
}
