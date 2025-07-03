//! Sv39: Page-Based 39-bit Virtual-Memory System

use super::{
    PageTableAddress, PageTableEntry, PageTableLevel, TransAddrError,
    constants::{PAGE_SIZE, PAGE_TABLE_LEN},
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

/// Translate gva to gpa in `Sv39`.
///
/// # Errors
/// This function will return an error if:
/// * An invalid Page Table Entry (PTE) is encountered during the page table walk.
/// * For a PTE that points to a superpage, remain PPN fields
///   that must be zero according to the specification is non-zero.
/// * The walk finishes all three levels of the page table hierarchy without reaching a leaf PTE.
///
/// # Panics
/// This function will panic if:
/// * The current `vsatp` register's mode is not `Sv39`.
#[allow(clippy::cast_possible_truncation)]
pub fn trans_addr(
    gva: GuestVirtualAddress,
) -> Result<GuestPhysicalAddress, (TransAddrError, &'static str)> {
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
        if pte.is_invalid() {
            return Err((
                TransAddrError::InvalidEntry,
                "Address translation failed: invalid pte",
            ));
        }

        if pte.is_leaf() {
            match level {
                PageTableLevel::Lv256TB | PageTableLevel::Lv512GB => unreachable!(),
                PageTableLevel::Lv1GB => {
                    if pte.ppn(0) != 0 {
                        return Err((
                            TransAddrError::InvalidEntry,
                            "Address translation failed: pte.ppn[0] != 0",
                        ));
                    }
                    if pte.ppn(1) != 0 {
                        return Err((
                            TransAddrError::InvalidEntry,
                            "Address translation failed: pte.ppn[1] != 0",
                        ));
                    }

                    return Ok(GuestPhysicalAddress(
                        (pte.ppn(2) << 30)
                            | (gva.vpn(1) << 21)
                            | (gva.vpn(0) << 12)
                            | gva.page_offset(),
                    ));
                }
                PageTableLevel::Lv2MB => {
                    if pte.ppn(0) != 0 {
                        return Err((
                            TransAddrError::InvalidEntry,
                            "Address translation failed: pte.ppn[0] != 0",
                        ));
                    }

                    return Ok(GuestPhysicalAddress(
                        (pte.ppn(2) << 30)
                            | (pte.ppn(1) << 21)
                            | (gva.vpn(0) << 12)
                            | gva.page_offset(),
                    ));
                }
                PageTableLevel::Lv4KB => {
                    return Ok(GuestPhysicalAddress(
                        (pte.ppn(2) << 30)
                            | (pte.ppn(1) << 21)
                            | (pte.ppn(0) << 12)
                            | gva.page_offset(),
                    ));
                }
            }
        }

        page_table_addr = PageTableAddress(pte.entire_ppn() as usize * PAGE_SIZE);
    }

    Err((
        TransAddrError::NoLeafEntry,
        "[sv39] cannnot reach to leaf entry",
    ))
}
