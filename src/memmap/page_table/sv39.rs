//! Sv39: Page-Based 39-bit Virtual-Memory System

use super::{
    constants::{PAGE_SIZE, PAGE_TABLE_LEN},
    PageTableAddress, PageTableEntry, PageTableLevel,
};
use crate::h_extension::csrs::vsatp;
use crate::memmap::{GuestPhysicalAddress, GuestVirtualAddress};

use core::slice::from_raw_parts_mut;

/// Pte field for Sv39x4
trait PteFieldSv39 {
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
}

/// Virtual address field for Sv39
trait AddressFieldSv39 {
    /// Return virtual page number
    fn vpn(self, index: usize) -> usize;
}

impl AddressFieldSv39 for GuestVirtualAddress {
    /// Return vpn value with index.
    fn vpn(self, index: usize) -> usize {
        match index {
            2 => (self.0 >> 30) & 0x1ff,
            1 => (self.0 >> 21) & 0x1ff,
            0 => (self.0 >> 12) & 0x1ff,
            _ => unreachable!(),
        }
    }
}

/// Translate gva to gpa in sv39
#[allow(clippy::cast_possible_truncation)]
pub fn trans_addr(gva: GuestVirtualAddress) -> GuestPhysicalAddress {
    let vsatp = vsatp::read();
    let mut page_table_addr = PageTableAddress(vsatp.ppn() << 12);
    assert!(matches!(vsatp.mode(), vsatp::Mode::Sv39));
    for level in [
        PageTableLevel::Lv1GB,
        PageTableLevel::Lv2MB,
        PageTableLevel::Lv4KB,
    ] {
        let page_table =
            unsafe { from_raw_parts_mut(page_table_addr.to_host_physical_ptr(), PAGE_TABLE_LEN) };
        let pte = page_table[gva.vpn(level as usize)];
        if pte.is_leaf() {
            match level {
                PageTableLevel::Lv256TB | PageTableLevel::Lv512GB => unreachable!(),
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
                        (pte.ppn(2) << 30)
                            | (gva.vpn(1) << 21)
                            | (gva.vpn(0) << 12)
                            | gva.page_offset(),
                    );
                }
                PageTableLevel::Lv2MB => {
                    assert!(
                        pte.ppn(0) == 0,
                        "Address translation failed: pte.ppn[0] != 0"
                    );
                    return GuestPhysicalAddress(
                        (pte.ppn(2) << 30)
                            | (pte.ppn(1) << 21)
                            | (gva.vpn(0) << 12)
                            | gva.page_offset(),
                    );
                }
                PageTableLevel::Lv4KB => {
                    return GuestPhysicalAddress(
                        (pte.ppn(2) << 30)
                            | (pte.ppn(1) << 21)
                            | (pte.ppn(0) << 12)
                            | gva.page_offset(),
                    );
                }
            }
        }

        page_table_addr = PageTableAddress(pte.entire_ppn() as usize * PAGE_SIZE);
    }

    unreachable!();
}
