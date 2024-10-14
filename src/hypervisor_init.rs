//! HS-mode level initialization.

use crate::device::MmioDevice;
use crate::guest::Guest;
use crate::h_extension::csrs::{
    hcounteren, hedeleg, hedeleg::ExceptionKind, henvcfg, hgatp, hideleg, hie, hstateen0, hstatus,
    hvip, vsatp, VsInterruptKind,
};
use crate::h_extension::instruction::hfence_gvma_all;
use crate::memmap::{
    constant::guest_memory, page_table::sv39x4::ROOT_PAGE_TABLE, GuestPhysicalAddress,
    HostPhysicalAddress,
};
use crate::trap::hypervisor_supervisor::hstrap_vector;
use crate::{HypervisorData, GUEST_DTB, HYPERVISOR_DATA};

use core::arch::asm;

use elf::{endian::AnyEndian, ElfBytes};
use riscv::register::{sepc, sie, sscratch, sstatus, sstatus::FS, stvec};

/// Entry point to HS-mode.
#[inline(never)]
pub extern "C" fn hstart(hart_id: usize, dtb_addr: usize) -> ! {
    // hart_id must be zero.
    assert_eq!(hart_id, 0);

    // dtb_addr test and hint for register usage.
    assert_ne!(dtb_addr, 0);

    // clear all hs-mode to vs-mode interrupts.
    hvip::clear(VsInterruptKind::External);
    hvip::clear(VsInterruptKind::Timer);
    hvip::clear(VsInterruptKind::Software);

    // disable address translation.
    vsatp::write(0);

    // enable all hs-mode interrupts
    unsafe {
        sie::set_sext();
        sie::set_ssoft();
        sie::set_stimer();
    }

    // set hie = 0x444
    hie::set(VsInterruptKind::External);
    hie::set(VsInterruptKind::Timer);
    hie::set(VsInterruptKind::Software);

    // enable Sstc extention
    henvcfg::set_stce();
    henvcfg::set_cde();
    henvcfg::set_cbze();
    henvcfg::set_cbcfe();

    // disable `ENVCFG` state
    hstateen0::all_state_set();
    hstateen0::clear_envcfg();

    // enable hypervisor counter
    hcounteren::set(0xffff_ffff);
    // enable supervisor counter
    unsafe {
        asm!("csrw scounteren, {bits}", bits = in(reg) 0xffff_ffff_u32);
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
        VsInterruptKind::External as usize
            | VsInterruptKind::Timer as usize
            | VsInterruptKind::Software as usize,
    );

    vsmode_setup(hart_id, HostPhysicalAddress(dtb_addr));
}

/// Setup for VS-mode
///
/// * Parse DTB
/// * Setup page table
fn vsmode_setup(hart_id: usize, dtb_addr: HostPhysicalAddress) -> ! {
    // create new guest data
    let guest_id = hart_id + 1;
    let guest_memory_begin = guest_memory::DRAM_BASE + guest_id * guest_memory::DRAM_SIZE_PER_GUEST;
    let root_page_table_addr = HostPhysicalAddress(ROOT_PAGE_TABLE.as_ptr() as usize);
    let new_guest = Guest::new(
        hart_id,
        &ROOT_PAGE_TABLE,
        &GUEST_DTB,
        guest_memory_begin..guest_memory_begin + guest_memory::DRAM_SIZE_PER_GUEST,
    );

    // parse device tree
    let device_tree = unsafe {
        match fdt::Fdt::from_ptr(dtb_addr.raw() as *const u8) {
            Ok(fdt) => fdt,
            Err(e) => panic!("{}", e),
        }
    };

    // initialize hypervisor data
    let mut hypervisor_data = unsafe { HYPERVISOR_DATA.lock() };
    hypervisor_data.get_or_init(|| HypervisorData::new(device_tree));

    // load guest elf from address
    let initrd = &hypervisor_data.get_mut().unwrap().devices().initrd;
    let guest_elf = unsafe {
        ElfBytes::<AnyEndian>::minimal_parse(core::slice::from_raw_parts(
            initrd.paddr().raw() as *mut u8,
            initrd.size(),
        ))
        .unwrap()
    };

    // load guest image
    let (guest_entry_point, elf_end_addr) =
        new_guest.load_guest_elf(&guest_elf, initrd.paddr().raw() as *mut u8);

    // filling remain memory region
    new_guest.filling_memory_region(
        elf_end_addr..guest_memory_begin + guest_memory::DRAM_SIZE_PER_GUEST,
    );

    // set device memory map
    hypervisor_data
        .get_mut()
        .unwrap()
        .devices()
        .device_mapping_g_stage(root_page_table_addr);

    // enable two-level address translation
    hgatp::set(hgatp::Mode::Sv39x4, 0, root_page_table_addr.raw() >> 12);
    hfence_gvma_all();

    // initialize IOMMU
    hypervisor_data.get_mut().unwrap().devices().init_iommu();

    // set new guest data
    hypervisor_data.get_mut().unwrap().register_guest(new_guest);

    unsafe {
        // sstatus.SUM = 1, sstatus.SPP = 0
        sstatus::set_sum();
        sstatus::set_spp(sstatus::SPP::Supervisor);
        // sstatus.sie = 1
        sstatus::set_sie();
        // sstatus.fs = 1
        sstatus::set_fs(FS::Initial);

        // hstatus.spv = 1 (enable V bit when sret executed)
        hstatus::set_spv();

        // set entry point
        sepc::write(guest_entry_point.raw());

        // set trap vector
        assert!(hstrap_vector as *const fn() as usize % 4 == 0);
        stvec::write(
            hstrap_vector as *const fn() as usize,
            stvec::TrapMode::Direct,
        );

        let mut context = hypervisor_data.get().unwrap().guest().context;
        context.set_sepc(sepc::read());

        // set sstatus value to context
        let mut sstatus_val;
        asm!("csrr {}, sstatus", out(reg) sstatus_val);
        context.set_sstatus(sstatus_val);
    }

    let guest_dtb_addr = hypervisor_data.get().unwrap().guest().guest_dtb_addr();

    // release HYPERVISOR_DATA lock
    drop(hypervisor_data);

    hart_entry(hart_id, guest_dtb_addr);
}

/// Entry for guest (VS-mode).
#[inline(never)]
fn hart_entry(hart_id: usize, dtb_addr: GuestPhysicalAddress) -> ! {
    // aquire hypervisor data
    let hypervisor_data = unsafe { HYPERVISOR_DATA.lock() };
    let stack_top = hypervisor_data.get().unwrap().guest().stack_top();
    // release HYPERVISOR_DATA lock
    drop(hypervisor_data);

    // init guest stack pointer is don't care
    sscratch::write(0);

    unsafe {
        // enter VS-mode
        asm!(
            ".align 4
            fence.i

            // set sp to scratch stack top
            mv sp, {stack_top}  
            addi sp, sp, -272 // Size of ContextData = 8 * 34

            // restore sstatus 
            ld t0, 32*8(sp)
            csrw sstatus, t0

            // restore pc
            ld t1, 33*8(sp)
            csrw sepc, t1

            // restore registers
            ld ra, 1*8(sp)
            ld gp, 3*8(sp)
            ld tp, 4*8(sp)
            ld t0, 5*8(sp)
            ld t1, 6*8(sp)
            ld t2, 7*8(sp)
            ld s0, 8*8(sp)
            ld s1, 9*8(sp)
            // a0 -> hart_id
            // a1 -> dtb_addr
            ld a2, 12*8(sp)
            ld a3, 13*8(sp)
            ld a4, 14*8(sp)
            ld a5, 15*8(sp)
            ld a6, 16*8(sp)
            ld a7, 17*8(sp)
            ld s2, 18*8(sp)
            ld s3, 19*8(sp)
            ld s4, 20*8(sp)
            ld s5, 21*8(sp)
            ld s6, 22*8(sp)
            ld s7, 23*8(sp)
            ld s8, 24*8(sp)
            ld s9, 25*8(sp)
            ld s10, 26*8(sp)
            ld s11, 27*8(sp)
            ld t3, 28*8(sp)
            ld t4, 29*8(sp)
            ld t5, 30*8(sp)
            ld t6, 31*8(sp)

            // swap HS-mode sp for original mode sp.
            addi sp, sp, 272
            csrrw sp, sscratch, sp

            sret
            ",
            in("a0") hart_id,
            in("a1") dtb_addr.raw(),
            stack_top = in(reg) stack_top.raw(),
            options(noreturn)
        );
    }
}
