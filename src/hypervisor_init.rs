use crate::guest::Guest;
use crate::h_extension::csrs::{
    hedeleg, hedeleg::ExceptionKind, hgatp, hgatp::HgatpMode, hideleg, hideleg::InterruptKind,
    hvip, vsatp,
};
use crate::h_extension::instruction::hfence_gvma_all;
use crate::memmap::constant::{PAGE_TABLE_BASE, PAGE_TABLE_OFFSET_PER_HART};
use crate::memmap::device::Device;
use crate::memmap::{page_table, page_table::PteFlag, DeviceMemmap, MemoryMap};
use crate::trap::supervisor::strap_vector;
use crate::HYPERVISOR_DATA;
use core::arch::asm;
use riscv::register::{sie, stvec};

/// Create page tables in G-stage address translation.
///
/// TODO: Automatic generation of page tables according to guest OS address translation map.
fn setup_g_stage_page_table(page_table_start: usize) {
    use PteFlag::{Accessed, Dirty, Exec, Read, Valid, Write};
    let memory_map: [MemoryMap; 7] = [
        // uart
        MemoryMap::new(
            0x1000_0000..0x1000_0100,               // guest_physical_memory_range
            0x1000_0000..0x1000_0100,               // physical_memory_range
            &[Dirty, Accessed, Write, Read, Valid], // flags
        ),
        // Device tree
        MemoryMap::new(
            0xbfe0_0000..0xc000_0000,
            0xbfe0_0000..0xc000_0000,
            &[Dirty, Accessed, Write, Read, Valid],
        ),
        // TEXT (physical map)
        MemoryMap::new(
            0x8000_0000..0x8020_0000,
            0x8000_0000..0x8020_0000,
            &[Dirty, Accessed, Exec, Read, Valid],
        ),
        // RAM
        MemoryMap::new(
            0x8020_0000..0x8080_0000,
            0x8020_0000..0x8080_0000,
            &[Dirty, Accessed, Write, Read, Valid],
        ),
        // hypervisor RAM
        MemoryMap::new(
            0x9000_0000..0x9000_4000,
            0x9000_0000..0x9000_4000,
            &[Dirty, Accessed, Write, Read, Valid],
        ),
        // TEXT
        MemoryMap::new(
            0xffff_ffff_c000_0000..0xffff_ffff_c020_0000,
            0x8000_0000..0x8020_0000,
            &[Dirty, Accessed, Exec, Read, Valid],
        ),
        // RAM
        MemoryMap::new(
            0xffff_ffff_c020_0000..0xffff_ffff_c080_0000,
            0x8020_0000..0x8080_0000,
            &[Dirty, Accessed, Write, Read, Valid],
        ),
    ];
    page_table::sv39x4::generate_page_table(page_table_start, &memory_map, true);
}

#[inline(never)]
pub extern "C" fn hstart(hart_id: usize, dtb_addr: usize) -> ! {
    // hart_id must be zero.
    assert_eq!(hart_id, 0);

    // clear all hypervisor interrupts.
    hvip::write(0);

    // disable address translation.
    vsatp::write(0);

    // set sie = 0x222
    unsafe {
        sie::set_ssoft();
        sie::set_stimer();
        sie::set_sext();
    }

    // specify delegation exception kinds.
    hedeleg::write(
        ExceptionKind::InstructionAddressMissaligned as usize
            | ExceptionKind::Breakpoint as usize
            | ExceptionKind::EnvCallFromUorVU as usize
            | ExceptionKind::InstructionPageFault as usize
            | ExceptionKind::LoadPageFault as usize
            | ExceptionKind::StoreAmoPageFault as usize,
    );
    // specify delegation interrupt kinds.
    hideleg::write(
        InterruptKind::Vsei as usize | InterruptKind::Vsti as usize | InterruptKind::Vssi as usize,
    );

    hsmode_setup(hart_id, dtb_addr);
}

/// Setup for HS-mode
///
/// * Parse DTB
/// * Setup page table
fn hsmode_setup(hart_id: usize, dtb_addr: usize) -> ! {
    // guest data
    let guest = Guest::new(hart_id);

    // parse device tree
    let device_tree = unsafe {
        match fdt::Fdt::from_ptr(dtb_addr as *const u8) {
            Ok(fdt) => fdt,
            Err(e) => panic!("{}", e),
        }
    };
    let mmap = DeviceMemmap::new(device_tree);

    // setup G-stage page table
    let page_table_start = PAGE_TABLE_BASE + hart_id * PAGE_TABLE_OFFSET_PER_HART;
    setup_g_stage_page_table(page_table_start);

    // enable two-level address translation
    hgatp::set(HgatpMode::Sv39x4, 0, page_table_start >> 12);
    hfence_gvma_all();

    // set trap vector
    unsafe {
        stvec::write(
            strap_vector as *const fn() as usize,
            stvec::TrapMode::Direct,
        );
    }

    // load guest image
    guest.load_guest_elf(mmap.initrd.paddr() as *mut u8, mmap.initrd.size());

    hart_entry(hart_id, dtb_addr);
}

/// Entry to guest mode.
fn hart_entry(_hart_id: usize, dtb_addr: usize) -> ! {
    // enter HS-mode
    unsafe {
        HYPERVISOR_DATA.get().unwrap().context.load();
        asm!(
            "
            mv a1, {dtb_addr}
            sret
            ",
            dtb_addr = in(reg) dtb_addr,
            options(noreturn)
        );
    }
}
