//! Register map for IOMMU.

use crate::memmap::HostPhysicalAddress;

/// IOMMU register map
pub struct IoMmuRegisters {
    /// A read-only register reporting features supported by the IOMMU.
    pub capabilities: Capabilities,
    /// Feature control register
    _fctl: u32,
    /// Designated For custom use
    _custom: u32,
    /// Device directory table pointer
    ddtp: u64,

    /// Command-queue base
    pub cqb: Cqb,
    /// Command-queue head
    _cqh: u32,
    /// Command-queue tail
    pub cqt: Cqt,

    /// Fault-queue base
    fqb: u64,
    /// Fault-queue head
    _fqh: u32,
    /// Fault-queue tail
    fqt: u32,

    /// Page-request-queue base
    pqb: u64,
    /// Page-request-queue head
    _pqh: u32,
    /// Page-request-queue tail
    pqt: u32,

    /// Command-queue CSR
    pub cqcsr: CqCsr,
    /// Fault-queue CSR
    fqcsr: u32,
    /// Page-request-queue CSR
    pqcsr: u32,
}

/// IOMMU capabilities
pub struct Capabilities(u64);
impl Capabilities {
    /// Return (major version, minor version)
    pub fn version(&self) -> (u8, u8) {
        (self.0 as u8 >> 4 & 0xf, self.0 as u8 & 0xf)
    }

    /// Is sv39x4 supported?
    pub fn is_sv39x4_supported(&self) -> bool {
        const FIELD_CAPABILITIES_SV39X4: usize = 17;
        self.0 >> FIELD_CAPABILITIES_SV39X4 & 0x1 == 1
    }
}

/// Command-queue base
pub struct Cqb(u64);
impl Cqb {
    /// set ppn value and log_2(size).
    pub fn set(&mut self, queue_addr: HostPhysicalAddress, size: usize) {
        // Is queue address aligned 4KiB?
        assert!(queue_addr % 4096 == 0);

        // CQB.PPN = B, CQB.LOG2SZ-1 = k - 1
        self.0 = (queue_addr.0 as u64 >> 12) << 10 | (size.ilog2() - 1) as u64;
    }
}

/// Command-queue tail
pub struct Cqt(u32);
impl Cqt {
    pub fn write(&mut self, value: u32) {
        self.0 = value;
    }
}

/// Command-queue CSR
pub struct CqCsr(u32);
impl CqCsr {
    /// set cqen (offset: 0) bit
    pub fn set_cqen(&mut self) {
        self.0 = self.0 | 1
    }

    /// cqon (offset: 16)
    pub fn cqon(&self) -> bool {
        const FIELD_CQCSR_CQON: usize = 0x10;
        self.0 >> FIELD_CQCSR_CQON & 0x1 == 1
    }
}
