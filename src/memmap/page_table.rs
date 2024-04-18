use alloc::boxed::Box;
use core::slice::from_raw_parts_mut;

use super::constant::PAGE_SIZE;
use super::MemoryMap;

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
        Self(ppn << 10 | flags as u64)
    }

    fn already_created(self) -> bool {
        self.0 & PteFlag::Valid as u64 == 1
    }

    fn pte(self) -> u64 {
        self.0 >> 10
    }
}

/// Generate third-level page table.
pub fn generate_page_table(root_table_start_addr: usize, memmaps: &[MemoryMap], initialize: bool) {
    const PAGE_TABLE_SIZE: usize = 512;

    let first_lv_page_table: &mut [PageTableEntry] = unsafe {
        from_raw_parts_mut(
            root_table_start_addr as *mut PageTableEntry,
            PAGE_TABLE_SIZE,
        )
    };

    // zero filling page table
    if initialize {
        first_lv_page_table.fill(PageTableEntry(0));
    }

    for memmap in memmaps {
        use crate::{print, println};
        println!("{:x?}", memmap.virt);

        assert!(memmap.virt.len() == memmap.phys.len());
        assert!(memmap.virt.start as usize % PAGE_SIZE == 0);
        assert!(memmap.phys.start as usize % PAGE_SIZE == 0);

        for offset in (0..memmap.virt.len()).step_by(PAGE_SIZE) {
            let v_start = memmap.virt.start + offset;
            let p_start = memmap.phys.start + offset;

            // first level
            let vpn2 = (v_start >> 30) & 0x1ff;
            if !first_lv_page_table[vpn2].already_created() {
                let second_pt = Box::new([0u64; PAGE_TABLE_SIZE]);
                let second_pt_paddr = Box::into_raw(second_pt);

                first_lv_page_table[vpn2] = PageTableEntry::new(
                    second_pt_paddr as u64 / PAGE_SIZE as u64,
                    PteFlag::Valid as u8,
                );
            }

            // second level
            let vpn1 = (v_start >> 21) & 0x1ff;
            let second_table_start_addr = first_lv_page_table[vpn2].pte() * PAGE_SIZE as u64;
            let second_lv_page_table: &mut [PageTableEntry] = unsafe {
                from_raw_parts_mut(
                    second_table_start_addr as *mut PageTableEntry,
                    PAGE_TABLE_SIZE,
                )
            };
            if !second_lv_page_table[vpn1].already_created() {
                let third_pt = Box::new([0u64; PAGE_TABLE_SIZE]);
                let third_pt_paddr = Box::into_raw(third_pt);

                second_lv_page_table[vpn1] = PageTableEntry::new(
                    third_pt_paddr as u64 / PAGE_SIZE as u64,
                    PteFlag::Valid as u8,
                );
            }

            // third level
            let vpn0 = (v_start >> 12) & 0x1ff;
            let third_table_start_addr = second_lv_page_table[vpn1].pte() * PAGE_SIZE as u64;
            let third_lv_page_table: &mut [PageTableEntry] = unsafe {
                from_raw_parts_mut(
                    third_table_start_addr as *mut PageTableEntry,
                    PAGE_TABLE_SIZE,
                )
            };
            third_lv_page_table[vpn0] =
                PageTableEntry::new((p_start / PAGE_SIZE).try_into().unwrap(), memmap.flags);
        }
    }
}
