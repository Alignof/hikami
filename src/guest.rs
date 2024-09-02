//! Guest data of each HARTs.

pub mod context;

use crate::memmap::page_table::sv39x4::FIRST_LV_PAGE_TABLE_LEN;
use crate::memmap::{
    constant::guest_memory,
    page_table,
    page_table::{constants::PAGE_SIZE, PageTableEntry, PteFlag},
    GuestPhysicalAddress, HostPhysicalAddress, MemoryMap,
};
use context::{Context, ContextData};

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::ops::Range;
use elf::{endian::AnyEndian, ElfBytes};

/// Aligned page size memory block
#[repr(C, align(0x1000))]
struct PageBlock([u8; 0x1000]);

impl PageBlock {
    fn alloc() -> HostPhysicalAddress {
        let mut host_physical_block_as_vec: Vec<core::mem::MaybeUninit<PageBlock>> =
            Vec::with_capacity(1);
        unsafe {
            host_physical_block_as_vec.set_len(1);
        }

        let host_physical_block_slice = host_physical_block_as_vec.into_boxed_slice();
        HostPhysicalAddress(Box::into_raw(host_physical_block_slice) as *const u8 as usize)
    }
}

/// Guest Information
#[derive(Debug)]
pub struct Guest {
    /// Guest ID
    #[allow(clippy::struct_field_names)]
    guest_id: usize,
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
        guest_dtb: &'static [u8; include_bytes!("../guest.dtb").len()],
        memory_region: Range<GuestPhysicalAddress>,
    ) -> Self {
        let stack_top_addr =
            unsafe { HostPhysicalAddress(core::ptr::addr_of!(crate::_stack_start) as usize) };
        let page_table_addr = HostPhysicalAddress(root_page_table.as_ptr() as usize);

        page_table::sv39x4::initialize_page_table(page_table_addr);

        let dtb_addr = Self::map_guest_dtb(hart_id, page_table_addr, guest_dtb);

        Guest {
            guest_id: hart_id,
            page_table_addr: HostPhysicalAddress(root_page_table.as_ptr() as usize),
            dtb_addr,
            stack_top_addr,
            memory_region,
            context: Context::new(stack_top_addr - core::mem::size_of::<ContextData>()),
        }
    }

    /// Map guest device tree region
    fn map_guest_dtb(
        hart_id: usize,
        page_table_addr: HostPhysicalAddress,
        guest_dtb: &'static [u8; include_bytes!("../guest.dtb").len()],
    ) -> GuestPhysicalAddress {
        use PteFlag::{Accessed, Dirty, Read, User, Valid, Write};

        assert!(guest_dtb.len() < guest_memory::GUEST_DTB_SIZE_PER_HART);

        let guest_dtb_gpa =
            guest_memory::DRAM_BASE + hart_id * guest_memory::GUEST_DTB_SIZE_PER_HART;
        let aligned_dtb_size = guest_dtb.len().div_ceil(PAGE_SIZE) * PAGE_SIZE;

        for offset in (0..aligned_dtb_size).step_by(PAGE_SIZE) {
            let guest_physical_addr = guest_dtb_gpa + offset;

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

        guest_dtb_gpa
    }

    /// Return HART(HARdware Thread) id.
    pub fn hart_id(&self) -> usize {
        self.guest_id
    }

    /// Return Stack top (end of memory region)
    pub fn stack_top(&self) -> HostPhysicalAddress {
        self.stack_top_addr
    }

    pub fn guest_dtb_addr(&self) -> GuestPhysicalAddress {
        self.dtb_addr
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
    pub fn load_guest_elf(
        &self,
        guest_elf: &ElfBytes<AnyEndian>,
        elf_addr: *mut u8,
    ) -> (GuestPhysicalAddress, GuestPhysicalAddress) {
        use PteFlag::{Accessed, Dirty, Exec, Read, User, Valid, Write};

        let align_size =
            |size: u64, align: u64| usize::try_from((size + (align - 1)) & !(align - 1)).unwrap();
        let mut elf_end: GuestPhysicalAddress = GuestPhysicalAddress::default();

        for prog_header in guest_elf
            .segments()
            .expect("failed to get segments from elf")
            .iter()
        {
            const PT_LOAD: u32 = 1;
            if prog_header.p_type == PT_LOAD && prog_header.p_filesz > 0 {
                assert!(prog_header.p_align >= PAGE_SIZE as u64);
                let aligned_segment_size = align_size(prog_header.p_filesz, prog_header.p_align);

                for offset in (0..aligned_segment_size).step_by(PAGE_SIZE) {
                    let guest_physical_addr =
                        self.dram_base() + prog_header.p_paddr.try_into().unwrap() + offset;
                    elf_end = core::cmp::max(elf_end, guest_physical_addr + PAGE_SIZE);

                    // allocate memory from heap
                    let aligned_page_size_block_addr = PageBlock::alloc();

                    // copy elf segment to new heap block
                    unsafe {
                        core::ptr::copy(
                            elf_addr.wrapping_add(
                                usize::try_from(prog_header.p_offset).unwrap() + offset,
                            ),
                            aligned_page_size_block_addr.raw() as *mut u8,
                            PAGE_SIZE,
                        );
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
                                // for dynamic patch
                                // ref: https://github.com/torvalds/linux/blob/67784a74e258a467225f0e68335df77acd67b7ab/arch/riscv/kernel/patch.c#L215C5-L215C21
                                // TODO: switch enable/disable write permission corresponding to VS-stage page table.
                                0b101 => &[Dirty, Accessed, Exec, Write, Read, User, Valid],
                                0b110 => &[Dirty, Accessed, Write, Read, User, Valid],
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
    pub fn filling_memory_region(&self, region: Range<GuestPhysicalAddress>) {
        use PteFlag::{Accessed, Dirty, Exec, Read, User, Valid, Write};

        let all_pte_flags_are_set = &[Dirty, Accessed, Exec, Write, Read, User, Valid];
        for guest_physical_addr in (region.start.raw()..region.end.raw()).step_by(PAGE_SIZE) {
            let guest_physical_addr = GuestPhysicalAddress(guest_physical_addr);

            // allocate memory from heap
            let aligned_page_size_block_addr = PageBlock::alloc();

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
