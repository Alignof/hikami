pub mod sv39;
pub mod sv39x4;

/// Size of memory areathat a page can point to.
pub const PAGE_SIZE: usize = 4096;

/// Page table level.
///
/// ref: The RISC-V Instruction Set Manual: Volume II p151.
#[derive(Copy, Clone)]
enum PageTableLevel {
    /// Page table level 0
    ///
    /// 1GB = 30 bit = vpn[1] (9 bit) + vpn[0] (9 bit) + offset (12 bit)
    Lv1GB,
    /// Page table level 1
    ///
    /// 2MB = 21 bit = vpn[0] (9 bit) + offset (12 bit)
    Lv2MB,
    /// Page table level 2
    ///
    /// 4KB = 12 bit = offset (12 bit)
    Lv4KB,
}

impl PageTableLevel {
    pub fn size(self) -> usize {
        match self {
            Self::Lv1GB => 0x40000000,
            Self::Lv2MB => 0x200000,
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
#[derive(Copy, Clone)]
struct PageTableEntry(u64);

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

/// Guest physical address (GPA)
struct GuestPhysicalAddress(usize);

impl GuestPhysicalAddress {
    fn vpn2(&self) -> usize {
        (self.0 >> 30) & 0x7ff
    }

    fn vpn1(&self) -> usize {
        (self.0 >> 21) & 0x1ff
    }

    fn vpn0(&self) -> usize {
        (self.0 >> 12) & 0x1ff
    }
}

/// Host physical address (GPA)
struct HostPhysicalAddress(usize);

impl HostPhysicalAddress {
    fn page_number(self, level: PageTableLevel) -> u64 {
        self.0 as u64 / level.size() as u64
    }
}
