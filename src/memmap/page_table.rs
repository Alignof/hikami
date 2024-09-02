//! Page table for address translation.

pub mod sv39x4;

use crate::memmap::{GuestPhysicalAddress, HostPhysicalAddress};

pub mod constants {
    /// Size of memory areathat a page can point to.
    pub const PAGE_SIZE: usize = 4096;
    /// Second or Third page table size
    ///
    /// vpn[1] == vpn[0] == 9 bit
    pub const PAGE_TABLE_LEN: usize = 512;
}

/// Page table level.
///
/// ref: The RISC-V Instruction Set Manual: Volume II p151.
#[derive(Copy, Clone, PartialEq)]
#[allow(clippy::module_name_repetitions)]
enum PageTableLevel {
    /// Page table level 0
    ///
    /// 1GB = 30 bit = vpn[1] (9 bit) + vpn[0] (9 bit) + offset (12 bit)
    Lv1GB = 2,
    /// Page table level 1
    ///
    /// 2MB = 21 bit = vpn[0] (9 bit) + offset (12 bit)
    Lv2MB = 1,
    /// Page table level 2
    ///
    /// 4KB = 12 bit = offset (12 bit)
    Lv4KB = 0,
}

impl PageTableLevel {
    pub fn size(self) -> usize {
        match self {
            Self::Lv1GB => 0x4000_0000,
            Self::Lv2MB => 0x0020_0000,
            Self::Lv4KB => 0x1000,
        }
    }
}

/// Each flags for page tables.
#[allow(dead_code)]
#[derive(Copy, Clone)]
pub enum PteFlag {
    /// PTE is valid.
    Valid = 0b0000_0001,
    /// PTE is readable.
    Read = 0b0000_0010,
    /// PTE is writable.
    Write = 0b0000_0100,
    /// PTE is executable.
    Exec = 0b0000_1000,
    /// The page may only accessed by U-mode software.
    User = 0b0001_0000,
    /// Global mapping.
    Global = 0b0010_0000,
    /// This page has been read, written or fetched.
    Accessed = 0b0100_0000,
    /// This page has been written.
    Dirty = 0b1000_0000,
}

/// Page table entry
#[derive(Copy, Clone, Default)]
#[allow(clippy::module_name_repetitions)]
pub struct PageTableEntry(u64);

impl PageTableEntry {
    fn new(ppn: u64, flags: u8) -> Self {
        Self(ppn << 10 | u64::from(flags))
    }

    fn already_created(self) -> bool {
        self.0 & PteFlag::Valid as u64 == 1
    }

    fn pte(self) -> u64 {
        self.0 >> 10
    }
}

/// Page table address
#[derive(Copy, Clone)]
struct PageTableAddress(usize);

impl From<*mut [PageTableEntry; constants::PAGE_TABLE_LEN]> for PageTableAddress {
    fn from(f: *mut [PageTableEntry; constants::PAGE_TABLE_LEN]) -> Self {
        PageTableAddress(f as *const u64 as usize)
    }
}

impl PageTableAddress {
    /// Return page number
    fn page_number(self) -> u64 {
        self.0 as u64 / constants::PAGE_SIZE as u64
    }

    /// Convert self to `PageTableEntry` pointer.
    fn to_pte_ptr(self) -> *mut PageTableEntry {
        self.0 as *mut PageTableEntry
    }
}

impl GuestPhysicalAddress {
    fn vpn(self, index: usize) -> usize {
        match index {
            2 => (self.0 >> 30) & 0x7ff,
            1 => (self.0 >> 21) & 0x1ff,
            0 => (self.0 >> 12) & 0x1ff,
            _ => unreachable!(),
        }
    }
}

impl HostPhysicalAddress {
    fn page_number(self) -> u64 {
        self.0 as u64 / constants::PAGE_SIZE as u64
    }
}
