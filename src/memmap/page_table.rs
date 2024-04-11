use alloc::boxed::Box;
use core::ops::Range;
use core::slice::from_raw_parts_mut;

use super::constant::PAGE_SIZE;
use super::Memmap;

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
    fn new(ppn: u64, flags: &[PteFlag]) -> Self {
        Self(ppn << 10 | flags.iter().fold(0, |pte_f, f| (pte_f | *f as u64)))
    }

    fn already_created(self) -> bool {
        self.0 & PteFlag::Valid as u64 == 1
    }

    fn pte(self) -> u64 {
        self.0 >> 10
    }
}

/// Generate second-level page table for now.
pub fn generate_page_table(
    root_table_start_addr: usize,
    memmap: &mut [(Range<usize>, Range<usize>, &[PteFlag])],
    _device_memap: Option<Memmap>,
) {
    const PTE_SIZE: usize = 8;
    const PAGE_TABLE_SIZE: usize = 512;
    let first_lv_page_table: &mut [PageTableEntry] = unsafe {
        from_raw_parts_mut(
            root_table_start_addr as *mut PageTableEntry,
            PAGE_TABLE_SIZE * PTE_SIZE,
        )
    };

    // zero filling page table
    first_lv_page_table.fill(PageTableEntry(0));

    for (v_range, p_range, pte_flags) in memmap {
        use crate::{print, println};
        println!("{:x?}", v_range);

        assert!(v_range.len() == p_range.len());
        assert!(v_range.start as usize % PAGE_SIZE == 0);
        assert!(p_range.start as usize % PAGE_SIZE == 0);

        const SECOND_LEVEL_STEP: usize = 0x2_0000;
        //for (v_start, p_start) in zip(v_range, p_range).step_by(PAGE_SIZE) {
        for offset in (0..v_range.len()).step_by(SECOND_LEVEL_STEP) {
            let v_start = v_range.start + offset;
            let p_start = p_range.start + offset;

            // first level
            let vpn2 = (v_start >> 30) & 0x1ff;
            if !first_lv_page_table[vpn2].already_created() {
                let second_pt = Box::new([0u64; PAGE_TABLE_SIZE]);
                let second_pt_paddr = Box::into_raw(second_pt);
                println!("second_pt_paddr: {:x}", second_pt_paddr as u64);

                first_lv_page_table[vpn2] = PageTableEntry::new(
                    second_pt_paddr as u64 / PAGE_SIZE as u64,
                    &[PteFlag::Valid],
                );
            }

            // second_level
            let vpn1 = (v_start >> 21) & 0x1ff;
            let second_table_start_addr = first_lv_page_table[vpn2].pte() * PAGE_SIZE as u64;
            let second_lv_page_table: &mut [PageTableEntry] = unsafe {
                from_raw_parts_mut(
                    second_table_start_addr as *mut PageTableEntry,
                    PAGE_TABLE_SIZE * PTE_SIZE,
                )
            };
            second_lv_page_table[vpn1] =
                PageTableEntry::new((p_start / PAGE_SIZE).try_into().unwrap(), pte_flags);
        }
    }
}
