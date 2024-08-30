//! Guest data of each HARTs.

pub mod context;

use crate::memmap::{
    page_table,
    page_table::{constants::PAGE_SIZE, PteFlag},
    GuestPhysicalAddress, HostPhysicalAddress, MemoryMap,
};
use context::Context;

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::mem::MaybeUninit;
use core::ops::Range;
use elf::{endian::AnyEndian, ElfBytes};

/// Guest Information
#[derive(Debug)]
pub struct Guest {
    /// Guest ID
    #[allow(clippy::struct_field_names)]
    guest_id: usize,
    /// Page table that is passed to guest address
    page_table_addr: HostPhysicalAddress,
    /// Device tree address
    dtb_addr: HostPhysicalAddress,
    /// Allocated memory region
    memory_region: Range<GuestPhysicalAddress>,
    /// Guest context data
    pub context: Context,
}

impl Guest {
    pub fn new(
        hart_id: usize,
        page_table_addr: HostPhysicalAddress,
        dtb_addr: HostPhysicalAddress,
        memory_region: Range<GuestPhysicalAddress>,
    ) -> Self {
        let stack_top = memory_region.end;
        Guest {
            guest_id: hart_id,
            page_table_addr,
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
    pub fn stack_top(&self) -> GuestPhysicalAddress {
        self.memory_region.end
    }

    pub fn guest_dtb_addr(&self) -> HostPhysicalAddress {
        self.dtb_addr
    }

    /// Return guest dram space start
    fn dram_base(&self) -> GuestPhysicalAddress {
        self.memory_region.start
    }

    fn page_size_iter(&self) -> core::iter::StepBy<Range<usize>> {
        (self.memory_region.start.raw()..self.memory_region.end.raw()).step_by(PAGE_SIZE)
    }

    /// Copy device tree from hypervisor side.  
    ///
    /// # Panics
    /// It will be panic if `dtb_addr` is invalid.
    pub unsafe fn copy_device_tree(&self, dtb_addr: HostPhysicalAddress, dtb_size: usize) {
        unsafe {
            core::ptr::copy(
                dtb_addr.raw() as *const u8,
                self.guest_dtb_addr().raw() as *mut u8,
                dtb_size,
            );
        }
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
        let all_pte_flags_are_set = &[Dirty, Accessed, Exec, Write, Read, User, Valid];
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
                let aligned_size = align_size(prog_header.p_filesz, prog_header.p_align);

                for offset in (0..aligned_size).step_by(PAGE_SIZE) {
                    let guest_physical_addr = self.dram_base() + offset;
                    elf_end = core::cmp::max(elf_end, guest_physical_addr + PAGE_SIZE);

                    // allocate memory from heap
                    let mut host_physical_block_as_vec: Vec<MaybeUninit<u8>> =
                        Vec::with_capacity(PAGE_SIZE);
                    unsafe {
                        host_physical_block_as_vec.set_len(PAGE_SIZE);
                    }
                    let host_physical_block_slice = host_physical_block_as_vec.into_boxed_slice();
                    let host_physical_block_begin =
                        HostPhysicalAddress(
                            Box::into_raw(host_physical_block_slice) as *const u8 as usize
                        );

                    // copy elf segment to new heap block
                    unsafe {
                        core::ptr::copy(
                            elf_addr.wrapping_add(
                                usize::try_from(prog_header.p_offset).unwrap() + offset,
                            ),
                            host_physical_block_begin.raw() as *mut u8,
                            usize::try_from(PAGE_SIZE).unwrap(),
                        );
                    }

                    // create memory mapping
                    page_table::sv39x4::generate_page_table(
                        self.page_table_addr,
                        &[MemoryMap::new(
                            guest_physical_addr..guest_physical_addr + PAGE_SIZE,
                            host_physical_block_begin..host_physical_block_begin + PAGE_SIZE,
                            all_pte_flags_are_set,
                        )],
                        false,
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
            let mut host_physical_block_as_vec: Vec<MaybeUninit<u8>> =
                Vec::with_capacity(PAGE_SIZE);
            unsafe {
                host_physical_block_as_vec.set_len(PAGE_SIZE);
            }
            let host_physical_block_slice = host_physical_block_as_vec.into_boxed_slice();
            let host_physical_block_begin =
                HostPhysicalAddress(Box::into_raw(host_physical_block_slice) as *const u8 as usize);

            // create memory mapping
            page_table::sv39x4::generate_page_table(
                self.page_table_addr,
                &[MemoryMap::new(
                    guest_physical_addr..guest_physical_addr + PAGE_SIZE,
                    host_physical_block_begin..host_physical_block_begin + PAGE_SIZE,
                    all_pte_flags_are_set,
                )],
                false,
            );
        }
    }
}
