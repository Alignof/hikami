//! Sv57: Page-Based 57-bit Virtual-Memory System

use super::{
    constants::{PAGE_SIZE, PAGE_TABLE_LEN},
    PageTableAddress, PageTableEntry, PageTableLevel,
};
use crate::h_extension::csrs::vsatp;
use crate::memmap::{GuestPhysicalAddress, GuestVirtualAddress};

use core::slice::from_raw_parts_mut;

/// Pte field for Sv57x4
trait PteFieldSv57 {
    /// Return entire ppn field
    fn ppn(self, index: usize) -> usize;
}

impl PteFieldSv57 for PageTableEntry {
    /// Return ppn
    #[allow(clippy::cast_possible_truncation)]
    fn ppn(self, index: usize) -> usize {
        match index {
            4 => (self.0 as usize >> 46) & 0x0ff, // 8 bit
            3 => (self.0 as usize >> 37) & 0x1ff, // 9 bit
            2 => (self.0 as usize >> 28) & 0x1ff, // 9 bit
            1 => (self.0 as usize >> 19) & 0x1ff, // 9 bit
            0 => (self.0 as usize >> 10) & 0x1ff, // 9 bit
            _ => unreachable!(),
        }
    }
}

/// Virtual address field for Sv57
trait AddressFieldSv57 {
    /// Return virtual page number
    fn vpn(self, index: usize) -> usize;
}

impl AddressFieldSv57 for GuestVirtualAddress {
    /// Return vpn value with index.
    fn vpn(self, index: usize) -> usize {
        match index {
            4 => (self.0 >> 48) & 0x1ff, // 9 bit
            3 => (self.0 >> 39) & 0x1ff, // 9 bit
            2 => (self.0 >> 30) & 0x1ff, // 9 bit
            1 => (self.0 >> 21) & 0x1ff, // 9 bit
            0 => (self.0 >> 12) & 0x1ff, // 9 bit
            _ => unreachable!(),
        }
    }
}

/// Translate gva to gpa in sv57
#[allow(clippy::cast_possible_truncation)]
pub fn trans_addr(gva: GuestVirtualAddress) -> Result<GuestPhysicalAddress, ()> {
    let vsatp = vsatp::read();
    assert!(matches!(vsatp.mode(), vsatp::Mode::Sv57));
    let mut page_table_addr = PageTableAddress(vsatp.ppn() << 12);

    for level in [
        PageTableLevel::Lv256TB,
        PageTableLevel::Lv512GB,
        PageTableLevel::Lv1GB,
        PageTableLevel::Lv2MB,
        PageTableLevel::Lv4KB,
    ] {
        let page_table =
            unsafe { from_raw_parts_mut(page_table_addr.to_host_physical_ptr(), PAGE_TABLE_LEN) };
        let pte = page_table[gva.vpn(level as usize)];
        if pte.is_leaf() {
            match level {
                PageTableLevel::Lv256TB => {
                    assert!(
                        pte.ppn(3) == 0,
                        "Address translation failed: pte.ppn[3] != 0"
                    );
                    assert!(
                        pte.ppn(2) == 0,
                        "Address translation failed: pte.ppn[2] != 0"
                    );
                    assert!(
                        pte.ppn(1) == 0,
                        "Address translation failed: pte.ppn[1] != 0"
                    );
                    assert!(
                        pte.ppn(0) == 0,
                        "Address translation failed: pte.ppn[0] != 0"
                    );
                    return Ok(GuestPhysicalAddress(
                        pte.ppn(4) << 48
                            | gva.vpn(3) << 39
                            | gva.vpn(2) << 30
                            | gva.vpn(1) << 21
                            | gva.vpn(0) << 12
                            | gva.page_offset(),
                    ));
                }
                PageTableLevel::Lv512GB => {
                    assert!(
                        pte.ppn(2) == 0,
                        "Address translation failed: pte.ppn[2] != 0"
                    );
                    assert!(
                        pte.ppn(1) == 0,
                        "Address translation failed: pte.ppn[1] != 0"
                    );
                    assert!(
                        pte.ppn(0) == 0,
                        "Address translation failed: pte.ppn[0] != 0"
                    );
                    return Ok(GuestPhysicalAddress(
                        pte.ppn(4) << 48
                            | pte.ppn(3) << 39
                            | gva.vpn(2) << 30
                            | gva.vpn(1) << 21
                            | gva.vpn(0) << 12
                            | gva.page_offset(),
                    ));
                }
                PageTableLevel::Lv1GB => {
                    assert!(
                        pte.ppn(1) == 0,
                        "Address translation failed: pte.ppn[1] != 0"
                    );
                    assert!(
                        pte.ppn(0) == 0,
                        "Address translation failed: pte.ppn[0] != 0"
                    );
                    return Ok(GuestPhysicalAddress(
                        pte.ppn(4) << 48
                            | pte.ppn(3) << 39
                            | pte.ppn(2) << 30
                            | gva.vpn(1) << 21
                            | gva.vpn(0) << 12
                            | gva.page_offset(),
                    ));
                }
                PageTableLevel::Lv2MB => {
                    assert!(
                        pte.ppn(0) == 0,
                        "Address translation failed: pte.ppn[0] != 0"
                    );
                    return Ok(GuestPhysicalAddress(
                        pte.ppn(4) << 48
                            | pte.ppn(3) << 39
                            | pte.ppn(2) << 30
                            | pte.ppn(1) << 21
                            | gva.vpn(0) << 12
                            | gva.page_offset(),
                    ));
                }
                PageTableLevel::Lv4KB => {
                    return Ok(GuestPhysicalAddress(
                        pte.ppn(4) << 48
                            | pte.ppn(3) << 39
                            | pte.ppn(2) << 30
                            | pte.ppn(1) << 21
                            | pte.ppn(0) << 12
                            | gva.page_offset(),
                    ));
                }
            }
        }

        page_table_addr = PageTableAddress(pte.entire_ppn() as usize * PAGE_SIZE);
    }

    Err(())
}
