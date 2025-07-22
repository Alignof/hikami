//! Guest data of each HARTs.

pub mod context;

use crate::memmap::page_table::sv39x4::FIRST_LV_PAGE_TABLE_LEN;
use crate::memmap::{
    constant::guest_memory,
    page_table,
    page_table::{constants::PAGE_SIZE, PageTableEntry, PteFlag},
    GuestPhysicalAddress, HostPhysicalAddress, MemoryMap,
};
use crate::PageBlock;
use context::{Context, ContextData};

use core::ops::Range;
use elf::{endian::AnyEndian, ElfBytes};

/// Guest Information
#[derive(Debug)]
pub struct Guest {
    /// HART ID
    hart_id: usize,
    /// Page table that is passed to guest address
    #[allow(dead_code)]
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
    #[must_use]
    pub fn new(
        hart_id: usize,
        root_page_table: &'static [PageTableEntry; FIRST_LV_PAGE_TABLE_LEN],
        guest_dtb: &'static [u8],
        guest_initrd: &'static [u8],
    ) -> Self {
        // Calculate guest memory region.
        let guest_memory_begin: GuestPhysicalAddress =
            guest_memory::DRAM_BASE + (hart_id + 1) * guest_memory::DRAM_SIZE_PER_GUEST;

        let memory_region =
            guest_memory_begin..guest_memory_begin + guest_memory::DRAM_SIZE_PER_GUEST;

        let stack_top_addr = HostPhysicalAddress(core::ptr::addr_of!(crate::_stack_start) as usize);
        let page_table_addr = HostPhysicalAddress(root_page_table.as_ptr() as usize);

        // init page table
        page_table::sv39x4::initialize_page_table(page_table_addr);

        // map guest memory space
        Self::allocate_memory_region(page_table_addr, &memory_region);

        // load guest dtb to memory
        let dtb_addr = Self::load_guest_dtb(hart_id, page_table_addr, guest_dtb);

        // laod guest initrd
        Self::load_initrd(hart_id, guest_initrd, &memory_region);

        Guest {
            hart_id,
            page_table_addr: HostPhysicalAddress(root_page_table.as_ptr() as usize),
            dtb_addr,
            stack_top_addr,
            memory_region,
            context: Context::new(stack_top_addr - core::mem::size_of::<ContextData>()),
        }
    }

    /// Allocate guest memory space from heap and create corresponding page table.
    fn allocate_memory_region(
        page_table_addr: HostPhysicalAddress,
        region: &Range<GuestPhysicalAddress>,
    ) {
        use PteFlag::{Accessed, Dirty, Exec, Read, User, Valid, Write};

        const G_STAGE_PTE_FLAGS: &[PteFlag; 7] = &[Dirty, Accessed, Read, Write, Exec, User, Valid];

        for guest_physical_addr in (region.start.raw()..region.end.raw()).step_by(PAGE_SIZE) {
            let guest_physical_addr = GuestPhysicalAddress(guest_physical_addr);

            // allocate memory from heap
            let aligned_page_size_block_addr: HostPhysicalAddress =
                if cfg!(feature = "identity_map") {
                    HostPhysicalAddress(guest_physical_addr.raw())
                } else {
                    PageBlock::alloc()
                };

            // create memory mapping
            page_table::sv39x4::generate_page_table(
                page_table_addr,
                &[MemoryMap::new(
                    guest_physical_addr..guest_physical_addr + PAGE_SIZE,
                    aligned_page_size_block_addr..aligned_page_size_block_addr + PAGE_SIZE,
                    G_STAGE_PTE_FLAGS,
                )],
            );
        }
    }

    /// Load guest's initrd
    fn load_initrd(
        hart_id: usize,
        guest_initrd: &'static [u8],
        memory_region: &Range<GuestPhysicalAddress>,
    ) {
        if guest_initrd.is_empty() {
            return;
        }

        let aligned_initrd_size = guest_initrd.len().div_ceil(PAGE_SIZE) * PAGE_SIZE;
        let initrd_start = memory_region.end - aligned_initrd_size;

        crate::println!(
            "initrd (hart {}): Mapping {:#x} bytes to GPA [{:#x} - {:#x}]",
            hart_id,
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
        hart_id: usize,
        page_table_addr: HostPhysicalAddress,
        guest_dtb: &'static [u8],
    ) -> GuestPhysicalAddress {
        use PteFlag::{Accessed, Dirty, Read, User, Valid, Write};

        assert!(guest_dtb.len() < guest_memory::GUEST_DTB_REGION_SIZE);

        // Guest device tree is loaded at a fixed offset from DRAM_BASE.
        let guest_dtb_addr =
            guest_memory::DRAM_BASE + hart_id * guest_memory::GUEST_DTB_REGION_SIZE;
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
    #[must_use]
    pub fn hart_id(&self) -> usize {
        self.hart_id
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

    /// Return guest dram space start
    #[must_use]
    pub fn memory_region(&self) -> &Range<GuestPhysicalAddress> {
        &self.memory_region
    }

    /// Return guest dram space start
    fn dram_base(&self) -> GuestPhysicalAddress {
        self.memory_region.start
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
    /// * `guest_initrd` - Initrd raw slice.
    ///
    /// # Panics
    /// Panics if it failed to calculate `aligned_segment_size` or failed to convert to usize.
    #[must_use]
    pub fn load_guest_elf(
        &self,
        guest_elf: &ElfBytes<AnyEndian>,
        elf_addr: *const u8,
    ) -> GuestPhysicalAddress {
        use PteFlag::{Accessed, Dirty, Read, User, Valid};

        /// Segment type `PT_LOAD`
        ///
        /// The array element specifies a loadable segment, described by `p_filesz` and `p_memsz`.
        const PT_LOAD: u32 = 1;

        let align_size =
            |size: u64, align: u64| usize::try_from((size + (align - 1)) & !(align - 1)).unwrap();

        for prog_header in guest_elf
            .segments()
            .expect("failed to get segments from elf")
            .iter()
        {
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
                        self.dram_base() + prog_header.p_paddr as usize + offset;

                    // Check if the target address is within the pre-allocated guest memory region
                    if !self.memory_region.contains(&guest_physical_addr) {
                        panic!(
                            "ELF segment paddr {:#x} out of guest memory region {:#x?}",
                            guest_physical_addr.raw(),
                            self.memory_region
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

                    // update page flags based on segment permissions
                    #[allow(clippy::match_same_arms)]
                    match prog_header.p_flags & 0b111 {
                        // R--
                        0b100 => page_table::sv39x4::update_page_flags(
                            guest_physical_addr,
                            [Dirty, Accessed, Read, User, Valid] // No Exec, No Write
                                .iter()
                                .fold(0, |pte_f, f| (pte_f | *f as u8)),
                        )
                        .expect("failed to update page flags"),
                        // Add Write permission to RX for dynamic patch
                        // ref: https://github.com/torvalds/linux/blob/67784a74e258a467225f0e68335df77acd67b7ab/arch/riscv/kernel/patch.c#L215C5-L215C21
                        // TODO: switch enable/disable write permission corresponding to VS-stage page table.
                        0b101 => (), // no update
                        // FIXME: Add Exec permission (RW -> RWX)
                        0b110 => (), // no update
                        0b111 => (), // no update
                        _ => panic!("unsupported ELF segment flags"),
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
        let phys_entry = self.dram_base().raw() as u64 + (virt_entry - virt_base);
        GuestPhysicalAddress(phys_entry as usize)
    }
}
