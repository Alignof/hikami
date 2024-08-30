//! Sv39: Page-Based 39-bit Virtual-Memory System
//!
//! [The RISC-V Instruction Set Manual: Volume II Version 20240411](https://github.com/riscv/riscv-isa-manual/releases/download/20240411/priv-isa-asciidoc.pdf)
//! pp.110-112

use alloc::boxed::Box;
use core::slice::from_raw_parts_mut;

use super::{constants::PAGE_SIZE, PageTableEntry, PteFlag};
use crate::memmap::{HostPhysicalAddress, MemoryMap};

/// Generate third-level page table. (Sv39)
#[allow(clippy::module_name_repetitions)]
pub fn generate_page_table(
    root_table_start_addr: HostPhysicalAddress,
    memmaps: &[MemoryMap],
    initialize: bool,
) {
    use crate::memmap::AddressRangeUtil;
    use crate::{print, println};

    const PAGE_TABLE_SIZE: usize = 512;

    let first_lv_page_table: &mut [PageTableEntry] = unsafe {
        from_raw_parts_mut(
            root_table_start_addr.raw() as *mut PageTableEntry,
            PAGE_TABLE_SIZE,
        )
    };

    // zero filling page table
    if initialize {
        first_lv_page_table.fill(PageTableEntry(0));
    }

    println!(
        "=========gen page table(Sv39): {:x}====================",
        root_table_start_addr.raw()
    );
    for memmap in memmaps {
        println!("{:x?} -> {:x?}", memmap.virt, memmap.phys);

        assert!(memmap.virt.len() == memmap.phys.len());
        assert!(memmap.virt.start.raw() % PAGE_SIZE == 0);
        assert!(memmap.phys.start % PAGE_SIZE == 0);

        for offset in (0..memmap.virt.len()).step_by(PAGE_SIZE) {
            let v_start = memmap.virt.start + offset;
            let p_start = memmap.phys.start + offset;

            // first level
            let vpn2 = (v_start.raw() >> 30) & 0x1ff;
            if !first_lv_page_table[vpn2].already_created() {
                let second_pt = Box::new([0u64; PAGE_TABLE_SIZE]);
                let second_pt_paddr = Box::into_raw(second_pt);

                first_lv_page_table[vpn2] = PageTableEntry::new(
                    second_pt_paddr as u64 / PAGE_SIZE as u64,
                    PteFlag::Valid as u8,
                );
            }

            // second level
            let vpn1 = (v_start.raw() >> 21) & 0x1ff;
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
            let vpn0 = (v_start.raw() >> 12) & 0x1ff;
            let third_table_start_addr = second_lv_page_table[vpn1].pte() * PAGE_SIZE as u64;
            let third_lv_page_table: &mut [PageTableEntry] = unsafe {
                from_raw_parts_mut(
                    third_table_start_addr as *mut PageTableEntry,
                    PAGE_TABLE_SIZE,
                )
            };
            third_lv_page_table[vpn0] = PageTableEntry::new(
                (p_start.raw() / PAGE_SIZE).try_into().unwrap(),
                memmap.flags,
            );
        }
    }
}
