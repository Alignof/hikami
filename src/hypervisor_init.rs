use crate::csrs::{
    hedeleg, hedeleg::ExceptionKind, hgatp, hgatp::HgatpMode, hideleg, hideleg::InterruptKind,
    hvip, vsatp,
};
use crate::memmap::constant::{PAGE_TABLE_BASE, PAGE_TABLE_OFFSET_PER_HART};
use crate::memmap::{page_table, page_table::PteFlag, MemoryMap};
use core::arch::asm;
use riscv::register::sie;

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

#[inline(always)]
fn hfence_gvma_all() {
    unsafe {
        asm!("hfence.gvma x0, x0");
    }
}

#[inline(never)]
pub extern "C" fn hstart(hart_id: usize, _dtb_addr: usize) {
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

    // setup G-stage page table
    let page_table_start = PAGE_TABLE_BASE + hart_id * PAGE_TABLE_OFFSET_PER_HART;
    setup_g_stage_page_table(page_table_start);

    // enable two-level address translation
    hgatp::set(HgatpMode::Sv39x4, 0, page_table_start >> 12);
    hfence_gvma_all();
}
