use crate::device::Device;
use crate::guest::Guest;
use crate::h_extension::csrs::{
    hedeleg, hedeleg::ExceptionKind, hgatp, hgatp::HgatpMode, hideleg, hstatus, hvip, vsatp,
    InterruptKind,
};
use crate::h_extension::instruction::hfence_gvma_all;
use crate::memmap::constant::{
    guest::{self, BASE_OFFSET_PER_HART},
    hypervisor::{self, PAGE_TABLE_OFFSET_PER_HART},
    DRAM_BASE,
};
use crate::trap::hypervisor_supervisor::hstrap_vector;
use crate::{GUEST_DTB, HYPERVISOR_DATA};
use core::arch::asm;

use elf::{endian::AnyEndian, ElfBytes};
use riscv::register::{sepc, sie, sscratch, sstatus, stvec};

#[inline(never)]
pub extern "C" fn hstart(hart_id: usize, dtb_addr: usize) -> ! {
    // hart_id must be zero.
    assert_eq!(hart_id, 0);

    // dtb_addr test and hint for register usage.
    assert_ne!(dtb_addr, 0);

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
    // aquire hypervisor data
    let mut hypervisor_data = unsafe { HYPERVISOR_DATA.lock() };

    // create new guest data
    let new_guest = Guest::new(hart_id);

    // parse device tree
    let device_tree = unsafe {
        match fdt::Fdt::from_ptr(dtb_addr as *const u8) {
            Ok(fdt) => fdt,
            Err(e) => panic!("{}", e),
        }
    };
    // copy device tree to guest
    let guest_dtb_addr = unsafe {
        new_guest.copy_device_tree(GUEST_DTB.as_ptr() as *const u8 as usize, GUEST_DTB.len())
    };

    // parsing and storing device data
    hypervisor_data.init_devices(device_tree);

    // setup G-stage page table
    let page_table_start = hypervisor::BASE_ADDR
        + hypervisor::PAGE_TABLE_OFFSET
        + hart_id * PAGE_TABLE_OFFSET_PER_HART;

    // load guest elf from address
    let guest_elf = unsafe {
        ElfBytes::<AnyEndian>::minimal_parse(core::slice::from_raw_parts(
            hypervisor_data.devices().initrd.paddr() as *mut u8,
            hypervisor_data.devices().initrd.size(),
        ))
        .unwrap()
    };

    // load guest image
    let guest_entry_point = new_guest.load_guest_elf(
        &guest_elf,
        hypervisor_data.devices().initrd.paddr() as *mut u8,
    );

    // crate page table from ELF
    new_guest.setup_g_stage_page_table_from_elf(&guest_elf, guest_entry_point, page_table_start);

    // set device memory map
    hypervisor_data
        .devices()
        .device_mapping_g_stage(page_table_start);

    // enable two-level address translation
    hgatp::set(HgatpMode::Sv39x4, 0, page_table_start >> 12);
    hfence_gvma_all();

    // set new guest data
    hypervisor_data.register_guest(new_guest);

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

        let mut context = hypervisor_data.guest().context;
        context.set_sepc(sepc::read());

        // set sstatus value to context
        let mut sstatus_val;
        asm!("csrr {}, sstatus", out(reg) sstatus_val);
        context.set_sstatus(sstatus_val);
    }

    // release HYPERVISOR_DATA lock
    drop(hypervisor_data);

    hart_entry(hart_id, guest_dtb_addr);
}

/// Entry for guest (VS-mode).
#[inline(never)]
fn hart_entry(hart_id: usize, dtb_addr: usize) -> ! {
    unsafe {
        // set stack top value to sscratch
        let guest_id = hart_id + 1;
        sscratch::write(DRAM_BASE + guest_id * BASE_OFFSET_PER_HART + guest::STACK_OFFSET);

        // enter VS-mode
        asm!(
            ".align 4
            fence.i

            // set to stack top
            li sp, 0x83000000
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
            ld a0, 10*8(sp)
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
            in("a1") dtb_addr,
            options(noreturn)
        );
    }
}
