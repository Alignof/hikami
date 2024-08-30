//! Sv39x4: Page-Based 39-bit Virtual-Memory System **in G-stage**.  
//! For guest physical address translation.
//!
//! [The RISC-V Instruction Set Manual: Volume II Version 20240411](https://github.com/riscv/riscv-isa-manual/releases/download/20240411/priv-isa-asciidoc.pdf)
//! p.151

use alloc::boxed::Box;
use core::slice::from_raw_parts_mut;

use super::{
    constants::PAGE_TABLE_SIZE, HostPhysicalAddress, PageTableAddress, PageTableEntry,
    PageTableLevel, PteFlag,
};
use crate::memmap::{GuestPhysicalAddress, MemoryMap};

/// First page table size
pub const FIRST_LV_PAGE_TABLE_SIZE: usize = 2048;

/// Generate third-level page table. (Sv39x4)
///
/// The number of address translation stages is determined by the size of the range.
#[allow(clippy::module_name_repetitions)]
pub fn generate_page_table(root_table_start_addr: usize, memmaps: &[MemoryMap], initialize: bool) {
    use crate::memmap::AddressRangeUtil;
    use crate::{print, println};

    assert!(root_table_start_addr % (16 * 1024) == 0); // root_table_start_addr must be aligned 16 KiB

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

        // decide page level from memory range
        let trans_page_level = match memmap.virt.len() {
            0x0..=0x1fffff => PageTableLevel::Lv4KB,
            0x200000..=0x3fffffff => PageTableLevel::Lv2MB,
            0x40000000..=usize::MAX => PageTableLevel::Lv1GB,
            _ => unreachable!(),
        };

        assert!(memmap.virt.start.raw() % trans_page_level.size() == 0);
        assert!(memmap.phys.start % trans_page_level.size() == 0);

        for offset in (0..memmap.virt.len()).step_by(trans_page_level.size()) {
            let v_start = memmap.virt.start + offset;
            let p_start = HostPhysicalAddress(memmap.phys.start + offset);

            let mut next_table_addr: PageTableAddress = PageTableAddress(0);
            for current_level in [
                PageTableLevel::Lv1GB,
                PageTableLevel::Lv2MB,
                PageTableLevel::Lv4KB,
            ] {
                let vpn = v_start.vpn(current_level as usize);
                let current_page_table = match current_level {
                    PageTableLevel::Lv1GB => &mut *first_lv_page_table,
                    PageTableLevel::Lv2MB | PageTableLevel::Lv4KB => unsafe {
                        from_raw_parts_mut(next_table_addr.to_pte_ptr(), PAGE_TABLE_SIZE)
                    },
                };

                // End of translation
                if current_level == trans_page_level {
                    current_page_table[vpn] =
                        PageTableEntry::new(p_start.page_number(), memmap.flags);

                    break;
                }

                // Create next level page table
                next_table_addr = if current_page_table[vpn].already_created() {
                    PageTableAddress(
                        current_page_table[vpn].pte() as usize * trans_page_level.size(),
                    )
                } else {
                    let next_page_table = Box::new([PageTableEntry::default(); PAGE_TABLE_SIZE]);
                    let next_page_table_addr: PageTableAddress =
                        Box::into_raw(next_page_table).into();

                    current_page_table[vpn] = PageTableEntry::new(
                        next_page_table_addr.page_number(),
                        PteFlag::Valid as u8,
                    );

                    next_page_table_addr
                };
            }
        }
    }
}
