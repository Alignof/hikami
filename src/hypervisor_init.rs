use crate::csrs::{hedeleg, hedeleg::ExceptionKind, hideleg, hideleg::InterruptKind, hvip, vsatp};
use crate::memmap::constant::{PAGE_TABLE_BASE, PAGE_TABLE_OFFSET_PER_HART};
use crate::memmap::{page_table::PteFlag, MemoryMap};
use riscv::register::sie;

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

    // init G-stage page tables
    use PteFlag::{Accessed, Dirty, Exec, Read, Valid, Write};
    let page_table_start = PAGE_TABLE_BASE + hart_id * PAGE_TABLE_OFFSET_PER_HART;
    let memory_map: [MemoryMap; 7] = [
        // (virtual_memory_range, physical_memory_range, flags),
        // uart
        MemoryMap::new(
            0x1000_0000..0x1000_0100,
            0x1000_0000..0x1000_0100,
            &[Dirty, Accessed, Write, Read, Valid],
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
}
