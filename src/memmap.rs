//! See `memmap/constant` module for specefic memmory map.

pub mod constant;
pub mod page_table;

use crate::memmap::page_table::PteFlag;
use core::ops::Range;

/// Utility for `Range<Address>`
trait AddressRangeUtil {
    /// Return length of range.
    fn len(&self) -> usize;
}

/// Guest Virtual Address
#[derive(Default, Debug, Copy, Clone)]
pub struct GuestVirtualAddress(pub usize);

/// Guest Physical Address
#[derive(Default, Debug, Copy, Clone, PartialEq, PartialOrd, Ord, Eq)]
pub struct GuestPhysicalAddress(pub usize);

impl GuestPhysicalAddress {
    /// Convert to usize.
    pub fn raw(self) -> usize {
        self.0
    }
}

impl core::ops::Add<usize> for GuestPhysicalAddress {
    type Output = GuestPhysicalAddress;
    fn add(self, other: usize) -> Self::Output {
        GuestPhysicalAddress(self.0 + other)
    }
}

impl core::ops::Sub<usize> for GuestPhysicalAddress {
    type Output = GuestPhysicalAddress;
    fn sub(self, other: usize) -> Self::Output {
        GuestPhysicalAddress(self.0 - other)
    }
}

impl core::ops::Rem<usize> for GuestPhysicalAddress {
    type Output = usize;
    fn rem(self, other: usize) -> Self::Output {
        self.0 % other
    }
}

impl AddressRangeUtil for Range<GuestPhysicalAddress> {
    fn len(&self) -> usize {
        self.end.raw() - self.start.raw()
    }
}

/// Host Physical Address
#[derive(Default, Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct HostPhysicalAddress(pub usize);

impl HostPhysicalAddress {
    /// Convert to usize.
    pub fn raw(self) -> usize {
        self.0
    }
}

impl core::ops::Add<usize> for HostPhysicalAddress {
    type Output = HostPhysicalAddress;
    fn add(self, other: usize) -> Self::Output {
        HostPhysicalAddress(self.0 + other)
    }
}

impl core::ops::Sub<usize> for HostPhysicalAddress {
    type Output = HostPhysicalAddress;
    fn sub(self, other: usize) -> Self::Output {
        HostPhysicalAddress(self.0 - other)
    }
}

impl core::ops::Rem<usize> for HostPhysicalAddress {
    type Output = usize;
    fn rem(self, other: usize) -> Self::Output {
        self.0 % other
    }
}

impl AddressRangeUtil for Range<HostPhysicalAddress> {
    fn len(&self) -> usize {
        self.end.raw() - self.start.raw()
    }
}

/// Struct for represent memory regtion.
#[derive(Debug, Clone)]
pub struct MemoryMap {
    /// Guest physical address
    virt: Range<GuestPhysicalAddress>,
    /// Host physical address
    pub phys: Range<HostPhysicalAddress>,
    /// Page table entry flags
    flags: u8,
}

impl MemoryMap {
    /// Create new `MemoryMap`.
    ///
    /// `flags` is mapped to bitmap.
    pub fn new(
        virt: Range<GuestPhysicalAddress>,
        phys: Range<HostPhysicalAddress>,
        flags: &[PteFlag],
    ) -> Self {
        Self {
            virt,
            phys,
            flags: flags.iter().fold(0, |pte_f, f| (pte_f | *f as u8)),
        }
    }
}
