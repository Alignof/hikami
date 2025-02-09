//! Page table for address translation.

pub mod sv39;
pub mod sv39x4;
pub mod sv57;

use crate::memmap::{GuestPhysicalAddress, GuestVirtualAddress, HostPhysicalAddress};

pub mod constants {
    //! Constants of page table.

    /// Size of memory areathat a page can point to.
    pub const PAGE_SIZE: usize = 4096;
    /// Second or Third page table size
    ///
    /// vpn\[1\] == vpn\[0\] == 9 bit
    pub const PAGE_TABLE_LEN: usize = 512;
}

/// Error of address translation
#[derive(Debug)]
pub enum TransAddrError {
    /// Invalid page table entry.
    InvalidEntry,
    /// Cannot reach leaf entry.
    NoLeafEntry,
}

/// Page table level.
///
/// ref: The RISC-V Instruction Set Manual: Volume II p151.
#[derive(Copy, Clone, PartialEq)]
#[allow(clippy::module_name_repetitions)]
enum PageTableLevel {
    /// 256TB = 48 bit = vpn\[3\] (9 bit) + vpn\[2\] (9 bit) + vpn\[1\] (9 bit) + vpn\[0\] (9 bit) + offset (12 bit)
    Lv256TB = 4,
    /// 512GB = 39 bit = vpn\[2\] (9 bit) + vpn\[1\] (9 bit) + vpn\[0\] (9 bit) + offset (12 bit)
    Lv512GB = 3,
    /// 1GB = 30 bit = vpn\[1\] (9 bit) + vpn\[0\] (9 bit) + offset (12 bit)
    Lv1GB = 2,
    /// 2MB = 21 bit = vpn\[0\] (9 bit) + offset (12 bit)
    Lv2MB = 1,
    /// 4KB = 12 bit = offset (12 bit)
    Lv4KB = 0,
}

impl PageTableLevel {
    /// Return usize.
    fn size(self) -> usize {
        match self {
            Self::Lv256TB => 0x1_0000_0000_0000,
            Self::Lv512GB => 0x80_0000_0000,
            Self::Lv1GB => 0x4000_0000,
            Self::Lv2MB => 0x0020_0000,
            Self::Lv4KB => 0x1000,
        }
    }
}

/// Heap memory region for page table.
#[repr(C, align(4096))]
struct PageTableMemory([PageTableEntry; constants::PAGE_TABLE_LEN]);

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
    /// Constructor for `PageTableEntry`.
    fn new(ppn: u64, flags: u8) -> Self {
        Self((ppn << 10) | u64::from(flags))
    }

    /// Is leaf page table entry
    fn is_leaf(self) -> bool {
        let pte_r = (self.0 >> 1) & 0x1;
        let pte_w = (self.0 >> 2) & 0x1;
        let pte_x = (self.0 >> 3) & 0x1;

        // For Zicfilp (TODO: remove it)
        pte_r == 1 || pte_x == 1 || (pte_r == 0 && pte_w == 1 && pte_x == 0)
    }

    /// Is pte invalid?
    fn is_invalid(self) -> bool {
        let pte_v = self.0 & 0x1;
        let pte_r = (self.0 >> 1) & 0x1;
        let pte_w = (self.0 >> 2) & 0x1;

        // For Zicfilp (TODO: remove it)
        pte_v == 0 || (pte_r == 0 && pte_w == 1)
    }

    /// Is it has already been created
    fn already_created(self) -> bool {
        self.0 & PteFlag::Valid as u64 == 1
    }

    /// Return entire ppn field
    fn entire_ppn(self) -> u64 {
        (self.0 >> 10) & 0xfff_ffff_ffff // 44 bit
    }
}

/// Page table address
#[derive(Copy, Clone)]
struct PageTableAddress(usize);

impl From<*mut PageTableMemory> for PageTableAddress {
    fn from(f: *mut PageTableMemory) -> Self {
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

    /// Convert guest physical page table address to host physical one.
    fn to_host_physical_ptr(self) -> *mut PageTableEntry {
        let hpa = g_stage_trans_addr(GuestPhysicalAddress(self.0)).unwrap();
        hpa.0 as *mut PageTableEntry
    }
}

impl GuestVirtualAddress {
    /// Return page offset.
    fn page_offset(self) -> usize {
        self.0 & 0xfff
    }
}

impl GuestPhysicalAddress {
    /// Return page offset.
    fn page_offset(self) -> usize {
        self.0 & 0xfff
    }
}

impl HostPhysicalAddress {
    /// Return page number
    fn page_number(self) -> u64 {
        self.0 as u64 / constants::PAGE_SIZE as u64
    }
}

/// VS-stage address translation.
pub fn vs_stage_trans_addr(
    gva: GuestVirtualAddress,
) -> Result<GuestPhysicalAddress, (TransAddrError, &'static str)> {
    use crate::h_extension::csrs::vsatp;

    let vsatp = vsatp::read();
    match vsatp.mode() {
        vsatp::Mode::Bare => unreachable!("no trans addr"),
        vsatp::Mode::Sv39 => sv39::trans_addr(gva),
        vsatp::Mode::Sv57 => sv57::trans_addr(gva),
        vsatp::Mode::Sv48 | vsatp::Mode::Sv64 => unimplemented!(),
    }
}

/// G-stage address translation.
pub fn g_stage_trans_addr(
    gpa: GuestPhysicalAddress,
) -> Result<HostPhysicalAddress, (TransAddrError, &'static str)> {
    use crate::h_extension::csrs::hgatp;

    let hgatp = hgatp::read();
    match hgatp.mode() {
        hgatp::Mode::Bare => unreachable!("no trans addr"),
        hgatp::Mode::Sv39x4 => sv39x4::trans_addr(gpa),
        hgatp::Mode::Sv48x4 | hgatp::Mode::Sv57x4 => unimplemented!(),
    }
}
