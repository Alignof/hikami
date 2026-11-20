//! Guest data of each HARTs.

pub mod context;

use crate::PageBlock;
use crate::board::Board;
use crate::memmap::{
    GuestPhysicalAddress, HostPhysicalAddress, MemoryMap,
    constant::guest_memory,
    page_table,
    page_table::{PteFlag, constants::PAGE_SIZE},
};
use context::{Context, ContextData};

use alloc::vec::Vec;
use core::ops::Range;
use elf::{ElfBytes, endian::AnyEndian};
use fdt::Fdt;

/// Guest Information
#[derive(Debug)]
pub struct Guest {
    /// HART ID
    hart_id: usize,
    /// Guest ID (min: 0)
    guest_hart_id: usize,
    /// Device tree address
    dtb_addr: GuestPhysicalAddress,
    /// Stack top address
    stack_top_addr: HostPhysicalAddress,
    /// Guest DRAM regions given by `/memory` of the guest device tree
    memory_regions: Vec<Range<GuestPhysicalAddress>>,
    /// Guest context data
    pub context: Context,
}

impl Guest {
    /// Initialize `Guest`.
    ///
    /// - Zero filling root page table.
    /// - Map guest dtb to guest memory space.
    #[must_use]
    pub fn new(
        hart_id: usize,
        guest_hart_id: usize,
        stack_top_addr: HostPhysicalAddress,
        board: &Board,
        reserved_regions: &[Range<usize>],
    ) -> Self {
        let guest_dtb = *board
            .guest_dtbs
            .get(hart_id)
            .expect("guest device tree for this hart is not found");
        let guest_device_tree = Fdt::new(guest_dtb).expect("failed to parse guest device tree");

        // Guest DRAM is defined by the guest device tree.
        let memory_regions: Vec<Range<GuestPhysicalAddress>> =
            crate::memmap::memory_regions(&guest_device_tree)
                .into_iter()
                .map(|region| GuestPhysicalAddress(region.start)..GuestPhysicalAddress(region.end))
                .collect();

        // map guest memory space
        for region in &memory_regions {
            crate::println!(
                "guest memory region (hart {}, guest id {}): GPA [{:#x} - {:#x}]",
                hart_id,
                guest_hart_id,
                region.start.raw(),
                region.end.raw(),
            );
            Self::allocate_memory_region(region, reserved_regions);
        }

        // load guest dtb to memory
        let dtb_addr = Self::load_guest_dtb(guest_hart_id, guest_dtb, board.guest_dtb_base);

        // laod guest initrd
        Self::load_initrd(guest_hart_id, board.guest_initrd, &guest_device_tree);

        Guest {
            hart_id,
            guest_hart_id,
            dtb_addr,
            stack_top_addr,
            memory_regions,
            context: Context::new(stack_top_addr - core::mem::size_of::<ContextData>()),
        }
    }

    /// Allocate guest memory space from heap and create corresponding page table.
    #[cfg(feature = "iommu")]
    fn allocate_memory_region(
        region: &Range<GuestPhysicalAddress>,
        _reserved_regions: &[Range<usize>],
    ) {
        use PteFlag::{Accessed, Dirty, Exec, Read, User, Valid, Write};

        const G_STAGE_PTE_FLAGS: &[PteFlag; 7] = &[Dirty, Accessed, Read, Write, Exec, User, Valid];

        for guest_physical_addr in (region.start.raw()..region.end.raw()).step_by(PAGE_SIZE) {
            let guest_physical_addr = GuestPhysicalAddress(guest_physical_addr);

            // allocate memory from heap
            let aligned_page_size_block_addr: HostPhysicalAddress = PageBlock::alloc();

            // create memory mapping
            page_table::sv39x4::generate_page_table(&[MemoryMap::new(
                guest_physical_addr..guest_physical_addr + PAGE_SIZE,
                aligned_page_size_block_addr..aligned_page_size_block_addr + PAGE_SIZE,
                G_STAGE_PTE_FLAGS,
            )]);
        }
    }

    /// Map guest memory space to the same host physical addresses.
    ///
    /// `reserved_regions` are excluded. The guest device tree must not use them as memory
    /// (e.g. list them in `/reserved-memory`).
    #[cfg(feature = "identity_map")]
    fn allocate_memory_region(
        region: &Range<GuestPhysicalAddress>,
        reserved_regions: &[Range<usize>],
    ) {
        use PteFlag::{Accessed, Dirty, Exec, Read, User, Valid, Write};
        const G_STAGE_PTE_FLAGS: &[PteFlag; 7] = &[Dirty, Accessed, Read, Write, Exec, User, Valid];

        let memory_maps: Vec<MemoryMap> = crate::memmap::subtract_ranges(
            &(region.start.raw()..region.end.raw()),
            reserved_regions,
        )
        .into_iter()
        .map(|part| {
            MemoryMap::new(
                GuestPhysicalAddress(part.start)..GuestPhysicalAddress(part.end),
                HostPhysicalAddress(part.start)..HostPhysicalAddress(part.end),
                G_STAGE_PTE_FLAGS,
            )
        })
        .collect();
        page_table::sv39x4::generate_page_table(&memory_maps);
    }

    /// Load guest's initrd to the address given by `/chosen` of the guest device tree.
    fn load_initrd(guest_hart_id: usize, guest_initrd: &'static [u8], guest_device_tree: &Fdt) {
        let Some(chosen) = guest_device_tree.find_node("/chosen") else {
            return;
        };
        let read_property = |name: &str| {
            chosen
                .property(name)
                .and_then(fdt::node::NodeProperty::as_usize)
        };
        let (Some(initrd_start), Some(initrd_end)) = (
            read_property("linux,initrd-start"),
            read_property("linux,initrd-end"),
        ) else {
            return;
        };
        let initrd_start = GuestPhysicalAddress(initrd_start);
        assert_eq!(
            initrd_end - initrd_start.raw(),
            guest_initrd.len(),
            "initrd size does not match `linux,initrd-start` and `linux,initrd-end`"
        );

        crate::println!(
            "initrd (guest id:{}): Mapping {:#x} bytes to GPA [{:#x} - {:#x}]",
            guest_hart_id,
            guest_initrd.len(),
            initrd_start.raw(),
            initrd_start.raw() + guest_initrd.len(),
        );

        for offset in (0..guest_initrd.len()).step_by(PAGE_SIZE) {
            let guest_physical_addr = initrd_start + offset;

            let page_size_block_addr: HostPhysicalAddress = if cfg!(feature = "identity_map") {
                // identity map
                HostPhysicalAddress(guest_physical_addr.raw())
            } else {
                // translate allocated address (GPA) -> HPA
                page_table::sv39x4::trans_addr(guest_physical_addr)
                    .expect("failed to translate guest memory address for initrd")
            };

            let copy_size = (guest_initrd.len() - offset).min(PAGE_SIZE);

            unsafe {
                core::ptr::copy_nonoverlapping(
                    guest_initrd.as_ptr().add(offset),
                    page_size_block_addr.raw() as *mut u8,
                    copy_size,
                );
            }
        }
    }

    /// Load guest device tree and create corresponding page table
    fn load_guest_dtb(
        guest_hart_id: usize,
        guest_dtb: &'static [u8],
        guest_dtb_base: GuestPhysicalAddress,
    ) -> GuestPhysicalAddress {
        use PteFlag::{Accessed, Dirty, Read, User, Valid, Write};

        assert!(guest_dtb.len() < guest_memory::GUEST_DTB_REGION_SIZE);

        // Guest device tree is loaded at a fixed offset from DRAM_BASE.
        let guest_dtb_addr = guest_dtb_base + guest_hart_id * guest_memory::GUEST_DTB_REGION_SIZE;
        let aligned_dtb_size = guest_dtb.len().div_ceil(PAGE_SIZE) * PAGE_SIZE;

        for offset in (0..aligned_dtb_size).step_by(PAGE_SIZE) {
            let guest_physical_addr = guest_dtb_addr + offset;

            let aligned_page_size_block_addr: HostPhysicalAddress = PageBlock::alloc();
            let copy_size = (guest_dtb.len() - offset).min(PAGE_SIZE);
            if copy_size > 0 {
                unsafe {
                    core::ptr::copy_nonoverlapping(
                        guest_dtb.as_ptr().add(offset),
                        aligned_page_size_block_addr.raw() as *mut u8,
                        copy_size,
                    );
                }
            }

            // create memory mapping
            page_table::sv39x4::generate_page_table(&[MemoryMap::new(
                guest_physical_addr..guest_physical_addr + PAGE_SIZE,
                aligned_page_size_block_addr..aligned_page_size_block_addr + PAGE_SIZE,
                // allow writing data to dtb to modify device tree on guest OS.
                &[Dirty, Accessed, Write, Read, User, Valid],
            )]);
        }

        guest_dtb_addr
    }

    /// Return HART(HARdware Thread) id.
    #[must_use]
    pub fn hart_id(&self) -> usize {
        self.hart_id
    }

    /// Return guest id.
    #[must_use]
    pub fn guest_hart_id(&self) -> usize {
        self.guest_hart_id
    }

    /// Return Stack top (end of memory region)
    #[must_use]
    pub fn stack_top(&self) -> HostPhysicalAddress {
        self.stack_top_addr
    }

    /// Return guest device tree address. (GPA)
    #[must_use]
    pub fn guest_dtb_addr(&self) -> GuestPhysicalAddress {
        self.dtb_addr
    }

    /// Return guest DRAM regions.
    #[must_use]
    pub fn memory_regions(&self) -> &[Range<GuestPhysicalAddress>] {
        &self.memory_regions
    }

    /// Load an elf to new allocated guest memory page.
    ///
    /// It only load `PT_LOAD` type segments.
    /// The entry segment is laoded at `kernel_base`.
    ///
    /// # Return
    /// - Entry point address in Guest memory space.
    /// - End address of the ELF. (for filling remind memory space)
    ///
    /// # Arguments
    /// * `guest_elf` - Elf loading guest space.
    /// * `elf_addr` - Elf address.
    /// * `kernel_base` - Address of loading kernel image
    ///
    /// # Safety
    /// This function will be safe if `elf_addr` is valid pointer.
    ///
    /// # Panics
    /// Panics if it failed to calculate `aligned_segment_size` or failed to convert to usize.
    #[must_use]
    pub unsafe fn load_guest_elf(
        &self,
        guest_elf: &ElfBytes<AnyEndian>,
        elf_addr: *const u8,
        kernel_base: GuestPhysicalAddress,
    ) -> GuestPhysicalAddress {
        /// Segment type `PT_LOAD`
        ///
        /// The array element specifies a loadable segment, described by `p_filesz` and `p_memsz`.
        const PT_LOAD: u32 = 1;

        let segments = guest_elf
            .segments()
            .expect("failed to get segments from elf");
        let entry_point = guest_elf.ehdr.e_entry;

        let entry_segment = segments
            .iter()
            .find(|s| s.p_vaddr <= entry_point && entry_point < s.p_vaddr + s.p_memsz)
            .unwrap_or_else(|| {
                panic!(
                    "Failed to find a segment containing the entry point {:#x}",
                    entry_point
                )
            });

        assert_eq!(
            entry_segment.p_type, PT_LOAD,
            "The segment containing the entry point {entry_point:#x} must be of PT_LOAD type",
        );

        let base_paddr = entry_segment.p_paddr;

        for s in segments.iter() {
            if s.p_type == PT_LOAD {
                assert!(
                    s.p_paddr >= base_paddr,
                    "Found a PT_LOAD segment at p_addr {:#x}, which is before the entry point segment's p_addr {:#x}",
                    s.p_paddr,
                    base_paddr
                );
            }
        }

        let align_size =
            |size: u64, align: u64| usize::try_from((size + (align - 1)) & !(align - 1)).unwrap();

        for prog_header in segments.iter() {
            if prog_header.p_type == PT_LOAD {
                // Skip segments that have no memory footprint.
                if prog_header.p_memsz == 0 {
                    continue;
                }

                assert!(prog_header.p_align >= PAGE_SIZE as u64);

                let aligned_segment_size = align_size(prog_header.p_memsz, prog_header.p_align);
                let segment_file_offset = usize::try_from(prog_header.p_offset).unwrap();
                let segment_file_size = usize::try_from(prog_header.p_filesz).unwrap();

                for offset in (0..aligned_segment_size).step_by(PAGE_SIZE) {
                    // Calculate the target GPA: Kernel's physical base + segment's physical offset + page offset
                    let guest_physical_addr =
                        kernel_base + (prog_header.p_paddr - base_paddr) as usize + offset;

                    // Check if the target address is within the guest memory regions
                    if !self
                        .memory_regions
                        .iter()
                        .any(|region| region.contains(&guest_physical_addr))
                    {
                        panic!(
                            "ELF segment paddr {:#x} out of guest memory regions {:#x?}",
                            guest_physical_addr.raw(),
                            self.memory_regions
                        );
                    }

                    // Translate the GPA to the HPA that was mapped in allocate_memory_region
                    let aligned_page_size_block_addr: HostPhysicalAddress = if cfg!(
                        feature = "identity_map"
                    ) {
                        HostPhysicalAddress(guest_physical_addr.raw())
                    } else {
                        page_table::sv39x4::trans_addr(guest_physical_addr).unwrap_or_else(|e| {
                                panic!(
                                    "failed to translate guest memory address {:#x} for ELF loading: {:?}",
                                    guest_physical_addr.raw(), e
                                )
                            })
                    };

                    // This logic calculates how many bytes to copy from the ELF file into the current page.
                    // It handles cases where a page is only partially covered by file data.
                    let copy_size = (segment_file_size
                        .saturating_sub(offset.min(segment_file_size)))
                    .min(PAGE_SIZE);
                    let copy_start = segment_file_offset + offset;

                    unsafe {
                        // Copy ELF segment data from the embedded binary
                        if copy_size > 0 {
                            core::ptr::copy_nonoverlapping(
                                elf_addr.add(copy_start),
                                aligned_page_size_block_addr.raw() as *mut u8,
                                copy_size,
                            );
                        }

                        // Zero-initialize the remaining part of the page if p_memsz > p_filesz
                        if copy_size < PAGE_SIZE {
                            core::ptr::write_bytes(
                                (aligned_page_size_block_addr.raw() as *mut u8).add(copy_size),
                                0,
                                PAGE_SIZE - copy_size,
                            );
                        }
                    }
                }
            }
        }

        // The virtual entry point.
        let virt_entry = guest_elf.ehdr.e_entry;
        // The virtual address of the first loadable segment is the virtual base.
        let virt_base = guest_elf
            .segments()
            .unwrap()
            .iter()
            .find(|p| p.p_type == PT_LOAD && p.p_memsz > 0)
            .map(|p| p.p_vaddr)
            .expect("No loadable segment found in ELF");

        // Calculate the physical entry point: physical_base + (virtual_entry - virtual_base)
        let phys_entry = kernel_base.raw() as u64 + (virt_entry - virt_base);
        GuestPhysicalAddress(phys_entry as usize)
    }
}
