//! Sv39x4: Page-Based 39-bit Virtual-Memory System **in G-stage**.
//! For guest physical address translation.
//!
//! [The RISC-V Instruction Set Manual: Volume II Version 20240411](https://github.com/riscv/riscv-isa-manual/releases/download/20240411/priv-isa-asciidoc.pdf) p.151

use super::{
    PageTableAddress, PageTableEntry, PageTableLevel, PageTableMemory, PteFlag, TransAddrError,
    constants::{PAGE_SIZE, PAGE_TABLE_LEN},
};
use crate::h_extension::csrs::hgatp;
use crate::memmap::{GuestPhysicalAddress, HostPhysicalAddress, MemoryMap};

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::ops::Range;
use core::slice::from_raw_parts_mut;

/// First page table size
pub const FIRST_LV_PAGE_TABLE_LEN: usize = 2048;

/// Device tree blob that is passed to guest
#[unsafe(link_section = ".root_page_table")]
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

/// Splits a single MemoryMap into multiple MemoryMaps to satisfy superpage alignment requirements.
///
/// This function takes a slice of MemoryMaps and returns a new Vec of MemoryMaps
/// where each one is properly aligned for the largest possible page size.
/// For example, a single large, unaligned region will be broken down into:
/// 1. A section mapped with 4KB pages to reach the first 2MB alignment boundary.
/// 2. A central section mapped with 2MB pages (or 1GB pages if possible).
/// 3. A final section mapped with 4KB pages for the remaining part.
fn split_memory_maps(memmaps: &[MemoryMap]) -> Vec<MemoryMap> {
    let mut split_maps = Vec::new();

    for memmap in memmaps {
        let mut current_virt = memmap.virt.start;
        let mut current_phys = memmap.phys.start;

        assert!(memmap.virt.start % 0x1000 == 0);
        assert!(memmap.phys.start % 0x1000 == 0);
        assert!(memmap.virt.end % 0x1000 == 0);
        assert!(memmap.phys.end % 0x1000 == 0);

        while current_virt < memmap.virt.end {
            let remaining_len = memmap.virt.end.raw() - current_virt.raw();

            // Determine the largest possible page size for the current address
            let (_page_level, page_size) = if remaining_len >= PageTableLevel::Lv1GB.size()
                && current_virt % PageTableLevel::Lv1GB.size() == 0
                && current_phys % PageTableLevel::Lv1GB.size() == 0
            {
                (PageTableLevel::Lv1GB, PageTableLevel::Lv1GB.size())
            } else if remaining_len >= PageTableLevel::Lv2MB.size()
                && current_virt % PageTableLevel::Lv2MB.size() == 0
                && current_phys % PageTableLevel::Lv2MB.size() == 0
            {
                (PageTableLevel::Lv2MB, PageTableLevel::Lv2MB.size())
            } else {
                (PageTableLevel::Lv4KB, PageTableLevel::Lv4KB.size())
            };

            split_maps.push(MemoryMap::new(
                current_virt..current_virt + page_size,
                current_phys..current_phys + page_size,
                &memmap.flags_as_array(),
            ));

            current_virt = current_virt + page_size;
            current_phys = current_phys + page_size;
        }
    }
    split_maps
}

/// Generate third-level page table. (Sv39x4)
///
/// The number of address translation stages is determined by the size of the range.
///
/// # Panics
/// Panics if `root_table_start_addr` is not aligned 16 Kib.
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

    // Split memory maps to ensure proper alignment for superpages.
    let aligned_memmaps = split_memory_maps(memmaps);

    for memmap in &aligned_memmaps {
        assert!(memmap.virt.len() == memmap.phys.len());

        // decide page level from memory range
        let trans_page_level = match memmap.virt.len() {
            0x0..=0x001f_ffff => PageTableLevel::Lv4KB,
            0x0020_0000..=0x3fff_ffff => PageTableLevel::Lv2MB,
            0x4000_0000..=usize::MAX => PageTableLevel::Lv1GB,
            _ => unreachable!(),
        };

        assert!(
            memmap.virt.start % trans_page_level.size() == 0,
            "memmap: {memmap:#x?}"
        );
        assert!(
            memmap.phys.start % trans_page_level.size() == 0,
            "memmap: {memmap:#x?}"
        );

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

/// Translate gva to gpa in `Sv39x4`.
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
/// * The current `vsatp` register's mode is not `Sv39x4`.
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

/// Updates the permission flags for a specific Guest Physical Address (GPA)
/// in the G-stage page table.
/// This function walks the page table to find the leaf PTE corresponding to the GPA
/// and overwrites its flags.
///
/// # Arguments
/// * `root_table_start_addr` - The Host Physical Address (HPA) of the root page table.
/// * `gpa` - The Guest Physical Address (GPA) of the page whose flags need to be updated.
/// * `new_flags` - The new set of permission flags to apply to the PTE.
///
/// # Errors
/// Returns an error if the page table walk encounters an invalid entry or
/// does not resolve to a leaf PTE for the given address.
#[allow(clippy::cast_possible_truncation)]
pub fn update_page_flags(
    gpa: GuestPhysicalAddress,
    new_flags: u8,
) -> Result<(), (TransAddrError, &'static str)> {
    let hgatp = hgatp::read();
    let mut page_table_addr = PageTableAddress(hgatp.ppn() << 12);

    // Iterate through the page table levels from top to bottom (L2 -> L1 -> L0).
    for level in [
        PageTableLevel::Lv1GB,
        PageTableLevel::Lv2MB,
        PageTableLevel::Lv4KB,
    ] {
        // Get the current level's page table. The root table has a different size.
        let page_table = match level {
            PageTableLevel::Lv1GB => unsafe {
                from_raw_parts_mut(page_table_addr.to_pte_ptr(), FIRST_LV_PAGE_TABLE_LEN)
            },
            _ => unsafe { from_raw_parts_mut(page_table_addr.to_pte_ptr(), PAGE_TABLE_LEN) },
        };

        // Get the Page Table Entry (PTE) for the current level's VPN.
        let vpn = gpa.vpn(level as usize);
        let pte = &mut page_table[vpn];

        if pte.is_invalid() {
            return Err((
                TransAddrError::InvalidEntry,
                "Update failed: encountered an invalid PTE during page table walk",
            ));
        }

        // If it's a leaf PTE (a superpage or a final 4KB page),
        // update its flags and terminate the walk.
        if pte.is_leaf() {
            pte.set_flags(new_flags);
            return Ok(());
        }

        // If not a leaf, it must point to the next level table.
        // Calculate the address of the next level page table.
        page_table_addr = PageTableAddress(pte.entire_ppn() as usize * PAGE_SIZE);
    }

    // If the loop completes without finding a leaf PTE, it's an error.
    Err((
        TransAddrError::NoLeafEntry,
        "Update failed: page table walk did not end on a leaf PTE",
    ))
}

/// Splits a superpage leaf PTE into a table of smaller pages.
///
/// This function is called when a part of a large page (1GB or 2MB) needs to be
/// unmapped. It replaces the single large leaf PTE with a pointer to a new,
/// next-level page table. This new table is then populated with leaf PTEs for
/// smaller pages that collectively map the same physical address range as the
/// original superpage.
///
/// # Arguments
/// * `pte` - A mutable reference to the superpage leaf PTE to be split.
/// * `level` - The page table level of the superpage PTE (e.g., Lv1GB, Lv2MB).
///
/// # Returns
/// * `Ok(())` on success.
/// * `Err` if memory for the new page table cannot be allocated or if an attempt
///   is made to split an unsplittable page (e.g., 4KB).
fn split_superpage(
    pte: &mut PageTableEntry,
    level: PageTableLevel,
) -> Result<(), (TransAddrError, &'static str)> {
    if !matches!(level, PageTableLevel::Lv1GB | PageTableLevel::Lv2MB) {
        // Only 1GB and 2MB pages can be split.
        return Err((
            TransAddrError::UnsupportedPageSize,
            "Attempted to split an unsplittable page level",
        ));
    }

    // Get original mapping info from the superpage PTE.
    let original_flags = (pte.0 & 0x3ff) as u8;
    let original_hpa_base = HostPhysicalAddress(pte.entire_ppn() as usize * PAGE_SIZE);

    // Allocate a new, zeroed page table for the next level down.
    let next_level_table = Box::new(PageTableMemory([PageTableEntry::default(); PAGE_TABLE_LEN]));
    let next_level_table_addr: PageTableAddress = Box::into_raw(next_level_table).into();

    // Get a mutable slice to the new table to populate it.
    let next_level_page_table: &mut [PageTableEntry] =
        unsafe { from_raw_parts_mut(next_level_table_addr.to_pte_ptr(), PAGE_TABLE_LEN) };

    // Determine the size of the smaller pages we are creating.
    let next_level_page_size = match level {
        PageTableLevel::Lv1GB => PageTableLevel::Lv2MB.size(),
        PageTableLevel::Lv2MB => PageTableLevel::Lv4KB.size(),
        _ => unreachable!(),
    };

    // Populate the new page table with leaf entries that map the original region.
    for i in 0..PAGE_TABLE_LEN {
        let hpa_offset = i * next_level_page_size;
        let next_hpa = original_hpa_base + hpa_offset;

        // Create a new leaf PTE with the original permissions.
        next_level_page_table[i] = PageTableEntry::new(next_hpa.page_number(), original_flags);
    }

    // Atomically update the original PTE to be a pointer to the new table.
    // The flags are now just 'Valid' because it's an intermediate PTE.
    *pte = PageTableEntry::new(next_level_table_addr.page_number(), PteFlag::Valid as u8);

    Ok(())
}

/// Invalidates the G-stage page table entries for a given range of Guest Physical Addresses.
///
/// This function walks the page table for each part of the specified range
/// and sets the corresponding leaf PTE to 0, effectively unmapping it.
/// If a superpage (1GB or 2MB) covering part of the range is encountered, and the
/// invalidation range is smaller than the superpage, the superpage is split into
/// smaller pages. The process is then restarted for the same address to traverse
/// down the newly created page table structure.
///
/// # Arguments
/// * `gpa_range` - A `core::ops::Range<GuestPhysicalAddress>` to be invalidated.
///
/// # Returns
/// * `Ok(())` on success.
/// * An `Err` with `TransAddrError` and a descriptive message if the page table
///   structure is invalid (e.g., a walk completes without finding a leaf PTE).
///
/// # Panics
/// This function will panic if the current `hgatp` register's mode is not `Sv39x4`.
#[allow(clippy::cast_possible_truncation)]
pub fn invalidate_address_range(
    gpa_range: Range<GuestPhysicalAddress>,
) -> Result<(), (TransAddrError, &'static str)> {
    let hgatp = hgatp::read();
    assert!(matches!(hgatp.mode(), hgatp::Mode::Sv39x4));
    let root_page_table_addr = PageTableAddress(hgatp.ppn() << 12);

    let mut current_gpa = gpa_range.start;

    // Loop until we have processed the entire invalidation range.
    'main: while current_gpa < gpa_range.end {
        let mut page_table_addr = root_page_table_addr;

        // Walk the page table for the current address.
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
                _ => unsafe { from_raw_parts_mut(page_table_addr.to_pte_ptr(), PAGE_TABLE_LEN) },
            };

            let vpn = current_gpa.vpn(level as usize);

            // Handle out-of-bounds addresses for the current table level.
            if vpn >= page_table.len() {
                let page_size = level.size();
                current_gpa =
                    GuestPhysicalAddress((current_gpa.raw() & !(page_size - 1)) + page_size);
                continue 'main;
            }

            let pte = &mut page_table[vpn];

            if pte.is_invalid() {
                // This region is already unmapped. Skip past the area this PTE would cover.
                let page_size = level.size();
                current_gpa =
                    GuestPhysicalAddress((current_gpa.raw() & !(page_size - 1)) + page_size);
                continue 'main;
            }

            if pte.is_leaf() {
                let page_size = level.size();
                let page_start_gpa = GuestPhysicalAddress(current_gpa.raw() & !(page_size - 1));

                // Case 1: The entire page is fully contained within the invalidation range.
                // We can invalidate the whole PTE and advance past it.
                if gpa_range.start <= page_start_gpa
                    && (page_start_gpa + page_size) <= gpa_range.end
                {
                    *pte = PageTableEntry(0);
                    current_gpa = page_start_gpa + page_size;
                    continue 'main;
                }

                // Case 2: The invalidation range partially overlaps or is smaller than the page.
                if level == PageTableLevel::Lv4KB {
                    // Smallest unit. Since current_gpa is in the invalidation range,
                    // we invalidate this page.
                    *pte = PageTableEntry(0);
                    current_gpa = page_start_gpa + page_size;
                    continue 'main;
                }

                // It's a superpage that needs to be split.
                split_superpage(pte, level)?;
                // After splitting, do not advance current_gpa. Restart the walk from the root
                // for the same address, which will now descend into the newly created table.
                continue 'main;
            }

            // Not a leaf, so descend to the next level table.
            page_table_addr = PageTableAddress(pte.entire_ppn() as usize * PAGE_SIZE);
        }

        // If the for loop completes without finding a leaf or breaking, it's an error.
        return Err((
            TransAddrError::NoLeafEntry,
            "Invalid page table structure: walk did not resolve to a leaf",
        ));
    }

    Ok(())
}
