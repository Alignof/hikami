//! See `memmap/constant` module for specefic memmory map.

pub mod constant;
pub mod page_table;

use crate::memmap::page_table::PteFlag;

use alloc::vec::Vec;
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
    #[must_use]
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
    #[must_use]
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
    #[must_use]
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
    /// Return flags as raw u8.
    pub fn flags(&self) -> u8 {
        self.flags
    }

    /// Return flags as an array of PteFlag.
    pub fn flags_as_array(&self) -> Vec<PteFlag> {
        let mut result = Vec::new();
        if self.flags & PteFlag::Valid as u8 != 0 {
            result.push(PteFlag::Valid);
        }
        if self.flags & PteFlag::Read as u8 != 0 {
            result.push(PteFlag::Read);
        }
        if self.flags & PteFlag::Write as u8 != 0 {
            result.push(PteFlag::Write);
        }
        if self.flags & PteFlag::Exec as u8 != 0 {
            result.push(PteFlag::Exec);
        }
        if self.flags & PteFlag::User as u8 != 0 {
            result.push(PteFlag::User);
        }
        if self.flags & PteFlag::Global as u8 != 0 {
            result.push(PteFlag::Global);
        }
        if self.flags & PteFlag::Accessed as u8 != 0 {
            result.push(PteFlag::Accessed);
        }
        if self.flags & PteFlag::Dirty as u8 != 0 {
            result.push(PteFlag::Dirty);
        }
        result
    }
}

impl From<fdt::standard_nodes::MemoryRegion> for MemoryMap {
    fn from(region: fdt::standard_nodes::MemoryRegion) -> Self {
        let size = region.size.unwrap();
        let virt_start = GuestPhysicalAddress(region.starting_address as usize);
        let phys_start = HostPhysicalAddress(region.starting_address as usize);
        MemoryMap::new(
            virt_start..virt_start + size,
            phys_start..phys_start + size,
            &[
                PteFlag::Dirty,
                PteFlag::Accessed,
                PteFlag::Write,
                PteFlag::Read,
                PteFlag::User,
                PteFlag::Valid,
            ],
        )
    }
}
