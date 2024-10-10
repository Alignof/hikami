use super::{
    constants::{PAGE_SIZE, PAGE_TABLE_LEN},
    PageTableAddress, PageTableEntry, PageTableLevel,
};
use crate::h_extension::csrs::vsatp;
use crate::memmap::{GuestPhysicalAddress, GuestVirtualAddress};

use core::slice::from_raw_parts_mut;

/// First page table size
pub const FIRST_LV_PAGE_TABLE_LEN: usize = 512;

/// Pte field for Sv39x4
trait PteFieldSv39 {
    /// Return ppn
    fn entire_ppn(self) -> u64;
    /// Return entire ppn field
    fn ppn(self, index: usize) -> usize;
}

impl PteFieldSv39 for PageTableEntry {
    /// Return ppn
    #[allow(clippy::cast_possible_truncation)]
    #[allow(dead_code)]
    fn ppn(self, index: usize) -> usize {
        match index {
            2 => (self.0 as usize >> 28) & 0x3ff_ffff, // 26 bit
            1 => (self.0 as usize >> 19) & 0x1ff,      // 9 bit
            0 => (self.0 as usize >> 10) & 0x1ff,      // 9 bit
            _ => unreachable!(),
        }
    }

    /// Return entire ppn field
    fn entire_ppn(self) -> u64 {
        (self.0 >> 10) & 0xfff_ffff_ffff // 44 bit
    }
}

/// Translate gva to gpa in sv39
#[allow(clippy::cast_possible_truncation)]
pub fn trans_addr(gpa: GuestVirtualAddress) -> GuestPhysicalAddress {
    let vsatp = vsatp::read();
    let mut page_table_addr = PageTableAddress(vsatp.ppn() << 12);
    assert!(matches!(vsatp.mode(), vsatp::Mode::Sv39));
    for level in [
        PageTableLevel::Lv1GB,
        PageTableLevel::Lv2MB,
        PageTableLevel::Lv4KB,
    ] {
        let page_table = match level {
            PageTableLevel::Lv1GB => unsafe {
                from_raw_parts_mut(page_table_addr.to_pte_ptr(), FIRST_LV_PAGE_TABLE_LEN)
            },
            PageTableLevel::Lv2MB | PageTableLevel::Lv4KB => unsafe {
                from_raw_parts_mut(page_table_addr.to_pte_ptr(), PAGE_TABLE_LEN)
            },
        };
        let pte = page_table[gpa.vpn(level as usize)];
        if pte.is_leaf() {
            match level {
                PageTableLevel::Lv1GB => {
                    assert!(
                        pte.ppn(0) == 0,
                        "Address translation failed: pte.ppn[0] != 0"
                    );
                    assert!(
                        pte.ppn(1) == 0,
                        "Address translation failed: pte.ppn[1] != 0"
                    );
                    return GuestPhysicalAddress(
                        pte.ppn(2) << 30 | gpa.vpn(1) << 21 | gpa.vpn(0) << 12 | gpa.page_offset(),
                    );
                }
                PageTableLevel::Lv2MB => {
                    assert!(
                        pte.ppn(0) == 0,
                        "Address translation failed: pte.ppn[0] != 0"
                    );
                    return GuestPhysicalAddress(
                        pte.ppn(2) << 30 | pte.ppn(1) << 21 | gpa.vpn(0) << 12 | gpa.page_offset(),
                    );
                }
                PageTableLevel::Lv4KB => {
                    return GuestPhysicalAddress(
                        pte.ppn(2) << 30 | pte.ppn(1) << 21 | pte.ppn(0) << 12 | gpa.page_offset(),
                    )
                }
            }
        }

        page_table_addr = PageTableAddress(pte.entire_ppn() as usize * PAGE_SIZE);
    }

    unreachable!();
}
