//! Register map for IOMMU.

use crate::memmap::HostPhysicalAddress;

/// IOMMU register map
#[repr(C)]
pub struct IoMmuRegisters {
    /// A read-only register reporting features supported by the IOMMU.
    pub capabilities: Capabilities,
    /// Feature control register
    _fctl: u32,
    /// Designated For custom use
    _custom: u32,
    /// Device directory table pointer
    pub ddtp: Ddtp,

    /// Command-queue base
    pub cqb: Cqb,
    /// Command-queue head
    _cqh: u32,
    /// Command-queue tail
    pub cqt: Cqt,

    /// Fault-queue base
    pub fqb: Fqb,
    /// Fault-queue head
    _fqh: u32,
    /// Fault-queue tail
    pub fqt: Fqt,

    /// Page-request-queue base
    pub pqb: Pqb,
    /// Page-request-queue head
    _pqh: u32,
    /// Page-request-queue tail
    pub pqt: Pqt,

    /// Command-queue CSR
    pub cqcsr: CqCsr,
    /// Fault-queue CSR
    pub fqcsr: FqCsr,
    /// Page-request-queue CSR
    pub pqcsr: PqCsr,
}

/// IOMMU capabilities
pub struct Capabilities(u64);
impl Capabilities {
    /// Return (major version, minor version)
    pub fn version(&self) -> (u8, u8) {
        let version_reg = self.0;
        (((version_reg >> 4) & 0xf) as u8, (version_reg & 0xf) as u8)
    }

    /// Is base format?
    ///
    /// true -> base format
    /// false -> extended format
    pub fn is_base_format(&self) -> bool {
        (self.0 >> 22) & 0x1 == 0
    }

    /// Is sv39x4 supported?
    pub fn is_sv39x4_supported(&self) -> bool {
        /// Field `Sv39x4` of `capabilities` register.
        const FIELD_CAPABILITIES_SV39X4: usize = 17;

        (self.0 >> FIELD_CAPABILITIES_SV39X4) & 0x1 == 1
    }
}

/// Command-queue base
pub struct Cqb(u64);
impl Cqb {
    /// set ppn value and `log_2(size`).
    pub fn set(&mut self, queue_addr: HostPhysicalAddress, size: usize) {
        // Is queue address aligned 4KiB?
        assert!(queue_addr % 4096 == 0);

        // CQB.PPN = B, CQB.LOG2SZ-1 = k - 1
        self.0 = ((queue_addr.0 as u64 >> 12) << 10) | u64::from(size.ilog2() - 1);
    }
}

/// Command-queue tail
pub struct Cqt(u32);
impl Cqt {
    /// Write a value.
    pub fn write(&mut self, value: u32) {
        self.0 = value;
    }
}

/// Command-queue CSR
pub struct CqCsr(u32);
impl CqCsr {
    /// set cqen (offset: 0) bit
    pub fn set_cqen(&mut self) {
        self.0 |= 1;
    }

    /// Return `cqon` field value. (offset: 16)
    pub fn cqon(&self) -> bool {
        /// Field `cqon` of `cqcsr` register. (16 bit)
        const FIELD_CQCSR_CQON: usize = 0x10;

        let cqcsr = self.0;
        (cqcsr >> FIELD_CQCSR_CQON) & 0x1 == 1
    }
}

/// Fault-queue base
pub struct Fqb(u64);
impl Fqb {
    /// set ppn value and `log_2(size`).
    pub fn set(&mut self, queue_addr: HostPhysicalAddress, size: usize) {
        // Is queue address aligned 4KiB?
        assert!(queue_addr % 4096 == 0);

        // FQB.PPN = B, FQB.LOG2SZ-1 = k - 1
        self.0 = ((queue_addr.0 as u64 >> 12) << 10) | u64::from(size.ilog2() - 1);
    }
}

/// Fault-queue tail
pub struct Fqt(u32);
impl Fqt {
    /// Write a value.
    pub fn write(&mut self, value: u32) {
        self.0 = value;
    }
}

/// Fault-queue CSR
pub struct FqCsr(u32);
impl FqCsr {
    /// set fqen (offset: 0) bit
    pub fn set_fqen(&mut self) {
        self.0 |= 1;
    }

    /// fqon (offset: 16)
    pub fn fqon(&self) -> bool {
        /// Field `fqon` of `fqcsr` register. (16 bit)
        const FIELD_FQCSR_FQON: usize = 0x10;
        let fqcsr = self.0;
        (fqcsr >> FIELD_FQCSR_FQON) & 0x1 == 1
    }
}

/// Page-request-queue base
pub struct Pqb(u64);
impl Pqb {
    /// set ppn value and `log_2(size`).
    pub fn set(&mut self, queue_addr: HostPhysicalAddress, size: usize) {
        // Is queue address aligned 4KiB?
        assert!(queue_addr % 4096 == 0);

        // PQB.PPN = B, PQB.LOG2SZ-1 = k - 1
        self.0 = ((queue_addr.0 as u64 >> 12) << 10) | u64::from(size.ilog2() - 1);
    }
}

/// Page-request-queue tail
pub struct Pqt(u32);
impl Pqt {
    /// Write a value.
    pub fn write(&mut self, value: u32) {
        self.0 = value;
    }
}

/// Page-request-queue CSR
pub struct PqCsr(u32);
impl PqCsr {
    /// set pqen (offset: 0) bit
    pub fn set_pqen(&mut self) {
        self.0 |= 1;
    }

    /// pqon (offset: 16)
    pub fn pqon(&self) -> bool {
        /// Field `pqon` of `pqcsr` register. (16 bit)
        const FIELD_PQCSR_PQON: usize = 0x10;

        let pqcsr = self.0;
        (pqcsr >> FIELD_PQCSR_PQON) & 0x1 == 1
    }
}

/// For `ddtp.iommu_mode`.
#[allow(dead_code)]
pub enum IoMmuMode {
    /// No inbound memory transactions are allowed by the IOMMU.
    Off,
    /// No translation or protection. All inbound memory accesses are passed through.
    Bare,
    /// One-level device-directory-table
    Lv1,
    /// Two-level device-directory-table
    Lv2,
    /// Three-level device-directory-table
    Lv3,
}

/// Device-directory-table pointer
pub struct Ddtp(u64);
impl Ddtp {
    /// set ppn and mode (defined in `IoMmuMode`).
    pub fn set(&mut self, mode: IoMmuMode, ddt_addr: HostPhysicalAddress) {
        /// Field `ppn` of `ddtp` register. (16 bit)
        const FIELD_DDTP_PPN: usize = 10;

        self.0 = ((ddt_addr.0 as u64 >> 12) << FIELD_DDTP_PPN) | mode as u64;
    }
}
