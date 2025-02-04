//! Sv39x4: Page-Based 39-bit Virtual-Memory System **in G-stage**.  
//! For guest physical address translation.
//!
//! [The RISC-V Instruction Set Manual: Volume II Version 20240411](https://github.com/riscv/riscv-isa-manual/releases/download/20240411/priv-isa-asciidoc.pdf) p.151

use super::{
    constants::{PAGE_SIZE, PAGE_TABLE_LEN},
    PageTableAddress, PageTableEntry, PageTableLevel, PageTableMemory, PteFlag, TransAddrError,
};
use crate::h_extension::csrs::hgatp;
use crate::memmap::{GuestPhysicalAddress, HostPhysicalAddress, MemoryMap};

use alloc::boxed::Box;
use core::slice::from_raw_parts_mut;

/// First page table size
pub const FIRST_LV_PAGE_TABLE_LEN: usize = 2048;

/// Device tree blob that is passed to guest
#[link_section = ".root_page_table"]
pub static ROOT_PAGE_TABLE: [PageTableEntry; FIRST_LV_PAGE_TABLE_LEN] =
    [PageTableEntry(0u64); FIRST_LV_PAGE_TABLE_LEN];

/// Pte field for Sv39x4
trait PteFieldSv39x4 {
    /// Return entire ppn field
    fn ppn(self, index: usize) -> usize;
}

impl PteFieldSv39x4 for PageTableEntry {
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
trait AddressFieldSv39x4 {
    /// Return virtual page number
    fn vpn(self, index: usize) -> usize;
}

impl AddressFieldSv39x4 for GuestPhysicalAddress {
    /// Return vpn value with index.
    fn vpn(self, index: usize) -> usize {
        match index {
            2 => (self.0 >> 30) & 0x7ff,
            1 => (self.0 >> 21) & 0x1ff,
            0 => (self.0 >> 12) & 0x1ff,
            _ => unreachable!(),
        }
    }
}

/// Zero filling root page table
pub fn initialize_page_table(root_table_start_addr: HostPhysicalAddress) {
    let first_lv_page_table: &mut [PageTableEntry] = unsafe {
        from_raw_parts_mut(
            root_table_start_addr.raw() as *mut PageTableEntry,
            FIRST_LV_PAGE_TABLE_LEN,
        )
    };

    // zero filling page table
    first_lv_page_table.fill(PageTableEntry(0));
}

/// Generate third-level page table. (Sv39x4)
///
/// The number of address translation stages is determined by the size of the range.
#[allow(clippy::module_name_repetitions)]
pub fn generate_page_table(root_table_start_addr: HostPhysicalAddress, memmaps: &[MemoryMap]) {
    use crate::memmap::AddressRangeUtil;

    assert!(root_table_start_addr % (16 * 1024) == 0); // root_table_start_addr must be aligned 16 KiB

    let first_lv_page_table: &mut [PageTableEntry] = unsafe {
        from_raw_parts_mut(
            root_table_start_addr.raw() as *mut PageTableEntry,
            FIRST_LV_PAGE_TABLE_LEN,
        )
    };

    for memmap in memmaps {
        assert!(memmap.virt.len() == memmap.phys.len());

        // decide page level from memory range
        let trans_page_level = match memmap.virt.len() {
            0x0..=0x001f_ffff => PageTableLevel::Lv4KB,
            0x0020_0000..=0x3fff_ffff => PageTableLevel::Lv2MB,
            0x4000_0000..=usize::MAX => PageTableLevel::Lv1GB,
            _ => unreachable!(),
        };

        assert!(memmap.virt.start % trans_page_level.size() == 0);
        assert!(memmap.phys.start % trans_page_level.size() == 0);

        for offset in (0..memmap.virt.len()).step_by(trans_page_level.size()) {
            let v_start = memmap.virt.start + offset;
            let p_start = memmap.phys.start + offset;

            let mut next_table_addr: PageTableAddress = PageTableAddress(0);
            for current_level in [
                PageTableLevel::Lv1GB,
                PageTableLevel::Lv2MB,
                PageTableLevel::Lv4KB,
            ] {
                let vpn = v_start.vpn(current_level as usize);
                let current_page_table = match current_level {
                    PageTableLevel::Lv256TB | PageTableLevel::Lv512GB => unreachable!(),
                    PageTableLevel::Lv1GB => &mut *first_lv_page_table,
                    PageTableLevel::Lv2MB | PageTableLevel::Lv4KB => unsafe {
                        from_raw_parts_mut(next_table_addr.to_pte_ptr(), PAGE_TABLE_LEN)
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
                        usize::try_from(current_page_table[vpn].entire_ppn()).unwrap() * PAGE_SIZE,
                    )
                } else {
                    let next_page_table =
                        Box::new(PageTableMemory([PageTableEntry::default(); PAGE_TABLE_LEN]));
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

/// Translate gpa to hpa in sv39x4
#[allow(clippy::cast_possible_truncation)]
pub fn trans_addr(
    gpa: GuestPhysicalAddress,
) -> Result<HostPhysicalAddress, (TransAddrError, &'static str)> {
    let hgatp = hgatp::read();
    let mut page_table_addr = PageTableAddress(hgatp.ppn() << 12);
    assert!(matches!(hgatp.mode(), hgatp::Mode::Sv39x4));
    for level in [
        PageTableLevel::Lv1GB,
        PageTableLevel::Lv2MB,
        PageTableLevel::Lv4KB,
    ] {
        let page_table = match level {
            PageTableLevel::Lv256TB | PageTableLevel::Lv512GB => unreachable!(),
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
                PageTableLevel::Lv256TB | PageTableLevel::Lv512GB => unreachable!(),
                PageTableLevel::Lv1GB => {
                    if pte.ppn(1) != 0 {
                        return Err((
                            TransAddrError::InvalidEntry,
                            "Address translation failed: pte.ppn[1] != 0",
                        ));
                    }
                    if pte.ppn(0) != 0 {
                        return Err((
                            TransAddrError::InvalidEntry,
                            "Address translation failed: pte.ppn[0] != 0",
                        ));
                    }

                    return Ok(HostPhysicalAddress(
                        (pte.ppn(2) << 30)
                            | (gpa.vpn(1) << 21)
                            | (gpa.vpn(0) << 12)
                            | gpa.page_offset(),
                    ));
                }
                PageTableLevel::Lv2MB => {
                    if pte.ppn(0) != 0 {
                        return Err((
                            TransAddrError::InvalidEntry,
                            "Address translation failed: pte.ppn[0] != 0",
                        ));
                    }

                    return Ok(HostPhysicalAddress(
                        (pte.ppn(2) << 30)
                            | (pte.ppn(1) << 21)
                            | (gpa.vpn(0) << 12)
                            | gpa.page_offset(),
                    ));
                }
                PageTableLevel::Lv4KB => {
                    return Ok(HostPhysicalAddress(
                        (pte.ppn(2) << 30)
                            | (pte.ppn(1) << 21)
                            | (pte.ppn(0) << 12)
                            | gpa.page_offset(),
                    ));
                }
            }
        }

        page_table_addr = PageTableAddress(pte.entire_ppn() as usize * PAGE_SIZE);
    }

    Err((
        TransAddrError::NoLeafEntry,
        "[sv39x4] cannnot reach to leaf entry",
    ))
}
