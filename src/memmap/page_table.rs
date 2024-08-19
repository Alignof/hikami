pub mod sv39;
pub mod sv39x4;

/// Size of memory areathat a page can point to.
pub const PAGE_SIZE: usize = 4096;

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
