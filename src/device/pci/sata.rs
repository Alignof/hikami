//! Serial ATA
//!
//! Ref: [https://osdev.jp/wiki/AHCI-Memo](https://osdev.jp/wiki/AHCI-Memo)

use super::Bdf;
use crate::memmap::HostPhysicalAddress;
use core::ops::Range;

/// SATA: Serial ATA
#[derive(Debug)]
pub struct Sata {
    /// Bus - device - function
    ident: Bdf,
    /// AHCI Base Address Register
    abar: Range<HostPhysicalAddress>,
    /// PCI Vender ID
    _vender_id: u32,
    /// PCI Device ID
    _device_id: u32,
}
