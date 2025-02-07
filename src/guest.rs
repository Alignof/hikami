//! Guest data of each HARTs.

pub mod context;

use crate::memmap::page_table::sv39x4::FIRST_LV_PAGE_TABLE_LEN;
use crate::memmap::{
    constant::guest_memory,
    page_table,
    page_table::{constants::PAGE_SIZE, PageTableEntry, PteFlag},
    GuestPhysicalAddress, HostPhysicalAddress, MemoryMap,
};
use crate::{PageBlock, GUEST_INITRD};
use context::{Context, ContextData};

use core::ops::Range;
use elf::{endian::AnyEndian, ElfBytes};

/// Guest Information
#[derive(Debug)]
pub struct Guest {
    /// HART ID
    hart_id: usize,
    /// Page table that is passed to guest address
    page_table_addr: HostPhysicalAddress,
    /// Device tree address
    dtb_addr: GuestPhysicalAddress,
    /// Stack top address
    stack_top_addr: HostPhysicalAddress,
    /// Allocated memory region
    memory_region: Range<GuestPhysicalAddress>,
    /// Guest context data
    pub context: Context,
}

impl Guest {
    /// Initialize `Guest`.
    ///
    /// - Zero filling root page table.
    /// - Map guest dtb to guest memory space.
    pub fn new(
        hart_id: usize,
        root_page_table: &'static [PageTableEntry; FIRST_LV_PAGE_TABLE_LEN],
        guest_dtb: &'static [u8; include_bytes!("../guest_image/guest.dtb").len()],
    ) -> Self {
        // calculate guest memory region
        let guest_memory_begin: GuestPhysicalAddress =
            guest_memory::DRAM_BASE + (hart_id + 1) * guest_memory::DRAM_SIZE_PER_GUEST;
        let memory_region =
            guest_memory_begin..guest_memory_begin + guest_memory::DRAM_SIZE_PER_GUEST;

        let stack_top_addr = HostPhysicalAddress(core::ptr::addr_of!(crate::_stack_start) as usize);
        let page_table_addr = HostPhysicalAddress(root_page_table.as_ptr() as usize);

        // init page table
        page_table::sv39x4::initialize_page_table(page_table_addr);

        // load guest dtb to memory
        let dtb_addr = if cfg!(feature = "identity_map") {
            GuestPhysicalAddress(guest_dtb.as_ptr() as usize)
        } else {
            Self::map_guest_dtb(hart_id, page_table_addr, guest_dtb)
        };

        Guest {
            hart_id,
            page_table_addr: HostPhysicalAddress(root_page_table.as_ptr() as usize),
            dtb_addr,
            stack_top_addr,
            memory_region,
            context: Context::new(stack_top_addr - core::mem::size_of::<ContextData>()),
        }
    }

    /// Load guest device tree and create corresponding page table
    ///
    /// Guest device tree will be placed start of guest memory region.
    fn map_guest_dtb(
        hart_id: usize,
        page_table_addr: HostPhysicalAddress,
        guest_dtb: &'static [u8; include_bytes!("../guest_image/guest.dtb").len()],
    ) -> GuestPhysicalAddress {
        use PteFlag::{Accessed, Dirty, Read, User, Valid, Write};

        assert!(guest_dtb.len() < guest_memory::GUEST_DTB_REGION_SIZE);

        // guest device tree is loaded at end of guest memory region.
        let guest_dtb_addr =
            guest_memory::DRAM_BASE + hart_id * guest_memory::GUEST_DTB_REGION_SIZE;
        let aligned_dtb_size = guest_dtb.len().div_ceil(PAGE_SIZE) * PAGE_SIZE;

        for offset in (0..aligned_dtb_size).step_by(PAGE_SIZE) {
            let guest_physical_addr = guest_dtb_addr + offset;

            // allocate memory from heap
            let aligned_page_size_block_addr = PageBlock::alloc();

            // copy elf segment to new heap block
            unsafe {
                core::ptr::copy(
                    guest_dtb.as_ptr().byte_add(offset),
                    aligned_page_size_block_addr.raw() as *mut u8,
                    PAGE_SIZE,
                );
            }

            // create memory mapping
            page_table::sv39x4::generate_page_table(
                page_table_addr,
                &[MemoryMap::new(
                    guest_physical_addr..guest_physical_addr + PAGE_SIZE,
                    aligned_page_size_block_addr..aligned_page_size_block_addr + PAGE_SIZE,
                    // allow writing data to dtb to modify device tree on guest OS.
                    &[Dirty, Accessed, Write, Read, User, Valid],
                )],
            );
        }

        guest_dtb_addr
    }

    /// Return HART(HARdware Thread) id.
    pub fn hart_id(&self) -> usize {
        self.hart_id
    }

    /// Return Stack top (end of memory region)
    pub fn stack_top(&self) -> HostPhysicalAddress {
        self.stack_top_addr
    }

    /// Return guest device tree address. (GPA)
    pub fn guest_dtb_addr(&self) -> GuestPhysicalAddress {
        self.dtb_addr
    }

    /// Return guest dram space start
    pub fn memory_region(&self) -> &Range<GuestPhysicalAddress> {
        &self.memory_region
    }

    /// Return guest dram space start
    fn dram_base(&self) -> GuestPhysicalAddress {
        self.memory_region.start
    }

    /// Load an elf to guest memory page.
    ///
    /// It only load `PT_LOAD` type segments.
    /// Entry address is base address of the dram.
    ///
    /// # Return
    /// - Entry point address in Guest memory space.
    /// - End address of the ELF. (for filling remind memory space)
    ///
    /// # Arguments
    /// * `guest_elf` - Elf loading guest space.
    /// * `elf_addr` - Elf address.
    #[cfg(feature = "identity_map")]
    pub fn load_guest_elf(
        &self,
        guest_elf: &ElfBytes<AnyEndian>,
        elf_addr: *const u8,
    ) -> (GuestPhysicalAddress, GuestPhysicalAddress) {
        /// Segment type `PT_LOAD`
        ///
        /// The array element specifies a loadable segment, described by `p_filesz` and `p_memsz`.
        const PT_LOAD: u32 = 1;

        let mut elf_end: GuestPhysicalAddress = GuestPhysicalAddress::default();

        for prog_header in guest_elf
            .segments()
            .expect("failed to get segments from elf")
            .iter()
        {
            if prog_header.p_type == PT_LOAD {
                let segment_gpa = GuestPhysicalAddress(
                    guest_memory::DRAM_BASE.raw()
                        + guest_memory::DRAM_SIZE_PER_GUEST * (self.hart_id + 1)
                        + prog_header.p_paddr as usize,
                );
                elf_end = core::cmp::max(
                    elf_end,
                    segment_gpa + prog_header.p_memsz as usize + PAGE_SIZE,
                );
                unsafe {
                    core::ptr::copy(
                        elf_addr.wrapping_add(prog_header.p_offset as usize) as *const u8,
                        segment_gpa.raw() as *mut u8,
                        prog_header.p_memsz as usize,
                    );
                }

                if prog_header.p_memsz > prog_header.p_filesz {
                    unsafe {
                        core::ptr::write_bytes(
                            elf_addr.wrapping_add(
                                prog_header.p_offset as usize + prog_header.p_filesz as usize,
                            ) as *mut u8,
                            0,
                            (prog_header.p_memsz - prog_header.p_filesz) as usize,
                        );
                    }
                }
            }
        }

        if !GUEST_INITRD.is_empty() {
            let aligned_initrd_size = GUEST_INITRD.len().div_ceil(PAGE_SIZE) * PAGE_SIZE;
            let initrd_start = guest_memory::DRAM_BASE
                + guest_memory::DRAM_SIZE_PER_GUEST * (self.hart_id + 1)
                - aligned_initrd_size;
            unsafe {
                core::ptr::copy(
                    GUEST_INITRD.as_ptr(),
                    initrd_start.raw() as *mut u8,
                    GUEST_INITRD.len(),
                );
            }
        }

        (self.dram_base(), elf_end)
    }

    /// Load an elf to new allocated guest memory page.
    ///
    /// It only load `PT_LOAD` type segments.
    /// Entry address is base address of the dram.
    ///
    /// # Return
    /// - Entry point address in Guest memory space.
    /// - End address of the ELF. (for filling remind memory space)
    ///
    /// # Arguments
    /// * `guest_elf` - Elf loading guest space.
    /// * `elf_addr` - Elf address.
    #[cfg(not(feature = "identity_map"))]
    pub fn load_guest_elf(
        &self,
        guest_elf: &ElfBytes<AnyEndian>,
        elf_addr: *const u8,
    ) -> (GuestPhysicalAddress, GuestPhysicalAddress) {
        /// Segment type `PT_LOAD`
        ///
        /// The array element specifies a loadable segment, described by `p_filesz` and `p_memsz`.
        const PT_LOAD: u32 = 1;

        use PteFlag::{Accessed, Dirty, Exec, Read, User, Valid, Write};

        let align_size =
            |size: u64, align: u64| usize::try_from((size + (align - 1)) & !(align - 1)).unwrap();
        let mut elf_end: GuestPhysicalAddress = GuestPhysicalAddress::default();

        for prog_header in guest_elf
            .segments()
            .expect("failed to get segments from elf")
            .iter()
        {
            if prog_header.p_type == PT_LOAD {
                assert!(prog_header.p_align >= PAGE_SIZE as u64);

                let aligned_segment_size = align_size(prog_header.p_memsz, prog_header.p_align);
                let segment_file_offset = usize::try_from(prog_header.p_offset).unwrap();
                let segment_file_size = usize::try_from(prog_header.p_filesz).unwrap();

                for offset in (0..aligned_segment_size).step_by(PAGE_SIZE) {
                    let guest_physical_addr =
                        self.dram_base() + prog_header.p_paddr.try_into().unwrap() + offset;
                    elf_end = core::cmp::max(elf_end, guest_physical_addr + PAGE_SIZE);

                    // allocate memory from heap
                    let aligned_page_size_block_addr = PageBlock::alloc();

                    // Determine the range of data to copy
                    let copy_start = segment_file_offset + offset;
                    let copy_size = if offset + PAGE_SIZE <= segment_file_size {
                        PAGE_SIZE
                    } else {
                        segment_file_size.saturating_sub(offset)
                    };

                    unsafe {
                        if copy_size > 0 {
                            // Copy ELF segment data from file
                            core::ptr::copy(
                                elf_addr.wrapping_add(copy_start),
                                aligned_page_size_block_addr.raw() as *mut u8,
                                copy_size,
                            );
                        }

                        if copy_size < PAGE_SIZE {
                            // Zero-initialize the remaining part of the page
                            core::ptr::write_bytes(
                                (aligned_page_size_block_addr.raw() as *mut u8).add(copy_size),
                                0,
                                PAGE_SIZE - copy_size,
                            );
                        }
                    }

                    // create memory mapping
                    page_table::sv39x4::generate_page_table(
                        self.page_table_addr,
                        &[MemoryMap::new(
                            guest_physical_addr..guest_physical_addr + PAGE_SIZE,
                            aligned_page_size_block_addr..aligned_page_size_block_addr + PAGE_SIZE,
                            match prog_header.p_flags & 0b111 {
                                0b100 => &[Dirty, Accessed, Read, User, Valid],
                                #[allow(clippy::match_same_arms)]
                                // Add Write permission to RX for dynamic patch
                                // ref: https://github.com/torvalds/linux/blob/67784a74e258a467225f0e68335df77acd67b7ab/arch/riscv/kernel/patch.c#L215C5-L215C21
                                // TODO: switch enable/disable write permission corresponding to VS-stage page table.
                                0b101 => &[Dirty, Accessed, Read, Write, Exec, User, Valid],
                                // FIXME: Add Exec permission (RW -> RWX)
                                0b110 => &[Dirty, Accessed, Read, Write, Exec, User, Valid],
                                0b111 => &[Dirty, Accessed, Exec, Write, Read, User, Valid],
                                _ => panic!("unsupported flags"),
                            },
                        )],
                    );
                }
            }
        }

        (self.dram_base(), elf_end)
    }

    /// Allocate guest memory space from heap and create corresponding page table.
    #[cfg(feature = "identity_map")]
    pub fn allocate_memory_region(&self, region: Range<GuestPhysicalAddress>) {
        use PteFlag::{Accessed, Dirty, Exec, Read, User, Valid, Write};

        let all_pte_flags_are_set = &[Dirty, Accessed, Exec, Write, Read, User, Valid];

        for guest_physical_addr in (region.start.raw()..region.end.raw()).step_by(PAGE_SIZE) {
            let guest_physical_addr = GuestPhysicalAddress(guest_physical_addr);
            let host_physical_addr = HostPhysicalAddress(guest_physical_addr.raw());
            // create memory mapping
            page_table::sv39x4::generate_page_table(
                self.page_table_addr,
                &[MemoryMap::new(
                    guest_physical_addr..guest_physical_addr + PAGE_SIZE,
                    host_physical_addr..host_physical_addr + PAGE_SIZE,
                    all_pte_flags_are_set,
                )],
            );
        }
    }

    /// Allocate guest memory space from heap and create corresponding page table.
    #[cfg(not(feature = "identity_map"))]
    pub fn allocate_memory_region(&self, region: Range<GuestPhysicalAddress>) {
        use PteFlag::{Accessed, Dirty, Exec, Read, User, Valid, Write};

        let all_pte_flags_are_set = &[Dirty, Accessed, Exec, Write, Read, User, Valid];

        let aligned_initrd_size = GUEST_INITRD.len().div_ceil(PAGE_SIZE) * PAGE_SIZE;
        let initrd_start = region.end - aligned_initrd_size;
        if !GUEST_INITRD.is_empty() {
            crate::println!(
                "initrd (GPA): {:#x}..{:#x}",
                initrd_start.raw(),
                initrd_start.raw() + GUEST_INITRD.len()
            );
        }

        for guest_physical_addr in (region.start.raw()..region.end.raw()).step_by(PAGE_SIZE) {
            let guest_physical_addr = GuestPhysicalAddress(guest_physical_addr);

            // allocate memory from heap
            let aligned_page_size_block_addr = PageBlock::alloc();

            // copy initrd to new heap block
            if (initrd_start..region.end).contains(&guest_physical_addr) {
                unsafe {
                    let offset = guest_physical_addr.raw() - initrd_start.raw();
                    core::ptr::copy(
                        GUEST_INITRD.as_ptr().byte_add(offset),
                        aligned_page_size_block_addr.raw() as *mut u8,
                        PAGE_SIZE,
                    );
                }
            }

            // create memory mapping
            page_table::sv39x4::generate_page_table(
                self.page_table_addr,
                &[MemoryMap::new(
                    guest_physical_addr..guest_physical_addr + PAGE_SIZE,
                    aligned_page_size_block_addr..aligned_page_size_block_addr + PAGE_SIZE,
                    all_pte_flags_are_set,
                )],
            );
        }
    }
}
