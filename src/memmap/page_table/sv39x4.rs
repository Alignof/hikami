//! Sv39x4: Page-Based 39-bit Virtual-Memory System **in G-stage**.  
//! For guest physical address translation.
//!
//! [The RISC-V Instruction Set Manual: Volume II Version 20240411](https://github.com/riscv/riscv-isa-manual/releases/download/20240411/priv-isa-asciidoc.pdf)
//! p.151

use alloc::boxed::Box;
use core::slice::from_raw_parts_mut;

use super::{PageTableEntry, PteFlag};
use crate::memmap::constant::PAGE_SIZE;
use crate::memmap::MemoryMap;

/// Generate third-level page table. (Sv39x4)
#[allow(clippy::module_name_repetitions)]
pub fn generate_page_table(root_table_start_addr: usize, memmaps: &[MemoryMap], initialize: bool) {
    use crate::{print, println};

    const FIRST_LV_PAGE_TABLE_SIZE: usize = 2048;
    const PAGE_TABLE_SIZE: usize = 512;

    let first_lv_page_table: &mut [PageTableEntry] = unsafe {
        from_raw_parts_mut(
            root_table_start_addr as *mut PageTableEntry,
            FIRST_LV_PAGE_TABLE_SIZE,
        )
    };

    // zero filling page table
    if initialize {
        first_lv_page_table.fill(PageTableEntry(0));
    }

    println!(
        "=========gen page table(Sv39x4): {:x}====================",
        root_table_start_addr
    );
    for memmap in memmaps {
        println!("{:x?} -> {:x?}", memmap.virt, memmap.phys);

        assert!(memmap.virt.len() == memmap.phys.len());
        assert!(memmap.virt.start % PAGE_SIZE == 0);
        assert!(memmap.phys.start % PAGE_SIZE == 0);

        for offset in (0..memmap.virt.len()).step_by(PAGE_SIZE) {
            let v_start = memmap.virt.start + offset;
            let p_start = memmap.phys.start + offset;

            // first level
            let vpn2 = (v_start >> 30) & 0x7ff;
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
