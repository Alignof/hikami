//! See `memmap/constant` module for specefic memmory map.

pub mod constant;
pub mod page_table;

use crate::memmap::page_table::PteFlag;
use core::ops::Range;

/// Guest Physical Address
pub struct GuestPhysicalAddress(usize);

#[derive(Clone)]
pub struct MemoryMap {
    virt: Range<usize>,
    phys: Range<usize>,
    flags: u8,
}

impl MemoryMap {
    pub fn new(virt: Range<usize>, phys: Range<usize>, flags: &[PteFlag]) -> Self {
        Self {
            virt,
            phys,
            flags: flags.iter().fold(0, |pte_f, f| (pte_f | *f as u8)),
        }
    }
}
