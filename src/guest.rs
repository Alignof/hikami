//! Guest data of each HARTs.

pub mod context;

use crate::memmap::constant::hypervisor;
use crate::memmap::{
    page_table,
    page_table::{PteFlag, PAGE_SIZE},
    MemoryMap,
};
use context::Context;
use core::ops::Range;

use alloc::boxed::Box;
use alloc::vec::Vec;
use elf::{endian::AnyEndian, ElfBytes};

/// Guest Information
#[derive(Debug)]
pub struct Guest {
    /// Guest ID
    guest_id: usize,
    /// Device tree address
    dtb_addr: usize,
    /// Allocated memory region
    memory_region: Range<usize>,
    /// Guest context data
    pub context: Context,
}

impl Guest {
    pub fn new(hart_id: usize, dtb_addr: usize, memory_region: Range<usize>) -> Self {
        let stack_top = memory_region.end;
        Guest {
            guest_id: hart_id,
            dtb_addr,
            memory_region,
            context: Context::new(stack_top),
        }
    }

    /// Return HART(HARdware Thread) id.
    pub fn hart_id(&self) -> usize {
        self.guest_id
    }

    /// Return Stack top (end of memory region)
    pub fn stack_top(&self) -> usize {
        self.memory_region.end
    }

    pub fn guest_dtb_addr(&self) -> usize {
        self.dtb_addr
    }

    /// Return guest dram space start
    fn dram_base(&self) -> usize {
        self.memory_region.start
    }

    /// Copy device tree from hypervisor side.  
    ///
    /// # Panics
    /// It will be panic if `dtb_addr` is invalid.
    pub unsafe fn copy_device_tree(&self, dtb_addr: usize, dtb_size: usize) {
        unsafe {
            core::ptr::copy(
                dtb_addr as *const u8,
                self.guest_dtb_addr() as *mut u8,
                dtb_size,
            );
        }
    }

    /// Load an elf to guest memory space.
    ///
    /// It only load `PT_LOAD` type segments.
    /// Entry address is determined by ... .
    ///
    /// # Arguments
    /// * `guest_elf` - Elf loading guest space.
    /// * `elf_addr` - Elf address.
    pub fn load_guest_elf(&self, guest_elf: &ElfBytes<AnyEndian>, elf_addr: *mut u8) -> usize {
        let guest_base_addr = self.dram_base();
        let first_segment_addr = guest_elf.segments().unwrap().iter().nth(0).unwrap().p_paddr;
        for prog_header in guest_elf
            .segments()
            .expect("failed to get segments from elf")
            .iter()
        {
            const PT_LOAD: u32 = 1;
            if prog_header.p_type == PT_LOAD && prog_header.p_filesz > 0 {
                unsafe {
                    core::ptr::copy(
                        elf_addr.wrapping_add(usize::try_from(prog_header.p_offset).unwrap()),
                        (guest_base_addr
                            + usize::try_from(prog_header.p_paddr - first_segment_addr).unwrap())
                            as *mut u8,
                        usize::try_from(prog_header.p_filesz).unwrap(),
                    );
                }
            }
        }

        guest_base_addr
    }

    /// Create page tables in G-stage address translation from ELF.
    pub fn setup_g_stage_page_table_from_elf(
        &self,
        guest_elf: &ElfBytes<AnyEndian>,
        page_table_start: usize,
    ) {
        use PteFlag::{Accessed, Dirty, Exec, Read, User, Valid, Write};

        let guest_base_addr = self.dram_base();
        let align_size = |size: u64, align: u64| ((size + (align - 1)) & !(align - 1)) as usize;
        let mut memory_map: Vec<MemoryMap> = Vec::new();
        let mut last_region: Range<usize> = Default::default();

        for prog_header in guest_elf
            .segments()
            .expect("failed to get segments from elf")
            .iter()
        {
            const PT_LOAD: u32 = 1;
            if prog_header.p_type == PT_LOAD && prog_header.p_filesz > 0 {
                let region_start: usize = guest_base_addr + prog_header.p_paddr as usize;
                let region_end: usize =
                    region_start + align_size(prog_header.p_memsz, prog_header.p_align);

                last_region = if last_region.end < region_end {
                    region_start..region_end
                } else {
                    last_region
                };

                memory_map.push(MemoryMap::new(
                    region_start..region_end, // virt
                    region_start..region_end, // phys
                    match prog_header.p_flags & 0b111 {
                        0b100 => &[Dirty, Accessed, Read, User, Valid],
                        0b101 => &[Dirty, Accessed, Exec, Read, User, Valid],
                        0b110 => &[Dirty, Accessed, Write, Read, User, Valid],
                        0b111 => &[Dirty, Accessed, Exec, Write, Read, User, Valid],
                        _ => panic!("unsupported flags"),
                    },
                ));
            }
        }

        memory_map.push(MemoryMap::new(
            last_region.end..0xffff_ffff, // virt
            last_region.end..0xffff_ffff, // phys
            &[Dirty, Accessed, Exec, Write, Read, User, Valid],
        ));
        page_table::sv39x4::generate_page_table(page_table_start, &memory_map, false);
    }

    /// Allocate guest memory space from heap and create corresponding page table.
    pub fn allocate_memory_space(&self) {
        for guest_physical_addr in self.memory_region {
            let mut guest_memory_as_vec = Vec::<u8>::with_capacity(PAGE_SIZE);
            unsafe {
                guest_memory_as_vec.set_len(PAGE_SIZE);
            }
            let guest_memory_slice = guest_memory_as_vec.into_boxed_slice();
            let guest_memory_begin = Box::into_raw(guest_memory_slice) as *const u8 as usize;
            page_table::sv39x4::generate_page_table(page_table_start, &memory_map, false);
        }
    }
}
