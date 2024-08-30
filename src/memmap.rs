//! See `memmap/constant` module for specefic memmory map.

pub mod constant;
pub mod page_table;

use crate::memmap::page_table::PteFlag;
use core::ops::Range;

/// Guest Physical Address
#[derive(Debug, Copy, Clone)]
pub struct GuestPhysicalAddress(usize);

impl GuestPhysicalAddress {
    pub fn raw(self) -> usize {
        self.0
    }
}

impl From<GuestPhysicalAddress> for usize {
    fn from(gpa: GuestPhysicalAddress) -> Self {
        gpa.0
    }
}

impl core::ops::Add<usize> for GuestPhysicalAddress {
    type Output = GuestPhysicalAddress;
    fn add(self, other: usize) -> Self::Output {
        GuestPhysicalAddress(self.0 + other)
    }
}

#[derive(Clone)]
pub struct MemoryMap {
    virt: Range<GuestPhysicalAddress>,
    phys: Range<usize>,
    flags: u8,
}

impl MemoryMap {
    pub fn new(virt: Range<GuestPhysicalAddress>, phys: Range<usize>, flags: &[PteFlag]) -> Self {
        Self {
            virt,
            phys,
            flags: flags.iter().fold(0, |pte_f, f| (pte_f | *f as u8)),
        }
    }
}
