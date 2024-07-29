use crate::guest::{self, Guest};
use crate::h_extension::csrs::{
    hedeleg, hedeleg::ExceptionKind, hgatp, hgatp::HgatpMode, hideleg, hstatus, hvip, vsatp,
    InterruptKind,
};
use crate::h_extension::instruction::hfence_gvma_all;
use crate::memmap::constant::{PAGE_TABLE_BASE, PAGE_TABLE_OFFSET_PER_HART};
use crate::memmap::device::Device;
use crate::memmap::{page_table, page_table::PteFlag, DeviceMemmap, MemoryMap};
use crate::trap::hypervisor_supervisor::hstrap_vector;
use crate::HYPERVISOR_DATA;
use core::arch::asm;
use riscv::register::{sepc, sie, sstatus, stvec};

/// Create page tables in G-stage address translation.
///
/// TODO: Automatic generation of page tables according to guest OS address translation map.
fn setup_g_stage_page_table(page_table_start: usize) {
    use PteFlag::{Accessed, Dirty, Exec, Read, User, Valid, Write};
    let memory_map: [MemoryMap; 2] = [
        // hypervisor RAM
        MemoryMap::new(
            0x9000_0000..0x9040_0000,
            0x9000_0000..0x9040_0000,
            &[Dirty, Accessed, Write, Read, User, Valid],
        ),
        // TEXT
        MemoryMap::new(
            0x9300_0000..0x9600_0000,
            0x9300_0000..0x9600_0000,
            &[Dirty, Accessed, Exec, Write, Read, User, Valid],
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

    vsmode_setup(hart_id, dtb_addr);
}

/// Setup for VS-mode
///
/// * Parse DTB
/// * Setup page table
fn vsmode_setup(hart_id: usize, dtb_addr: usize) -> ! {
    // guest data
    let new_guest = Guest::new(hart_id);

    // parse device tree
    let device_tree = unsafe {
        match fdt::Fdt::from_ptr(dtb_addr as *const u8) {
            Ok(fdt) => fdt,
            Err(e) => panic!("{}", e),
        }
    };
    let device_mmap = DeviceMemmap::new(device_tree);

    // setup G-stage page table
    let page_table_start = PAGE_TABLE_BASE + hart_id * PAGE_TABLE_OFFSET_PER_HART;
    setup_g_stage_page_table(page_table_start);
    device_mmap.device_mapping_g_stage(page_table_start);

    // enable two-level address translation
    hgatp::set(HgatpMode::Sv39x4, 0, page_table_start >> 12);
    hfence_gvma_all();

    // copy device tree
    let guest_dtb_addr = unsafe { new_guest.copy_device_tree(dtb_addr, device_tree.total_size()) };

    // load guest image
    let guest_entry_point = new_guest.load_guest_elf(
        device_mmap.initrd.paddr() as *mut u8,
        device_mmap.initrd.size(),
    );

    // store device data
    unsafe {
        let hypervisor_data = HYPERVISOR_DATA.lock();
        hypervisor_data.init_devices(dtb_addr, device_tree.total_size(), device_mmap);
        hypervisor_data.regsiter_guest(new_guest);
    }

    unsafe {
        // sstatus.SUM = 1, sstatus.SPP = 0
        sstatus::set_sum();
        sstatus::set_spp(sstatus::SPP::Supervisor);

        // hstatus.spv = 1 (enable V bit when sret executed)
        hstatus::set_spv();

        // set entry point
        sepc::write(guest_entry_point);

        // set trap vector
        assert!(hstrap_vector as *const fn() as usize % 4 == 0);
        stvec::write(
            hstrap_vector as *const fn() as usize,
            stvec::TrapMode::Direct,
        );

        // save current context data
        guest::context::store();
    }

    hart_entry(hart_id, guest_dtb_addr);
}

/// Entry to guest mode.
#[inline(never)]
fn hart_entry(_hart_id: usize, dtb_addr: usize) -> ! {
    // enter VS-mode
    unsafe {
        let mut hypervisor_data = HYPERVISOR_DATA.lock();
        hypervisor_data.guest.context.set_xreg(11, dtb_addr as u64); // a1 = dtb_addr
        drop(hypervisor_data); // release HYPERVISOR_DATA lock

        guest::context::load();
        asm!("sret", options(noreturn));
    }
}
