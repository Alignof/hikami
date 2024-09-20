//! Register map for IOMMU.

/// IOMMU register map
pub struct IoMmuRegisters {
    /// A read-only register reporting features supported by the IOMMU.
    capabilities: u64,
    /// Feature control register
    fctl: u32,
    /// Designated For custom use
    _custom: u32,
    /// Device directory table pointer
    ddtp: u64,

    /// Command-queue base
    cqb: u64,
    /// Command-queue head
    _cqh: u32,
    /// Command-queue tail
    cqt: u32,

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
    cqcsr: u32,
    /// Fault-queue CSR
    fqcsr: u32,
    /// Page-request-queue CSR
    pqcsr: u32,
}
