use crate::memmap::constant::{
    DRAM_BASE, DRAM_SIZE_PAR_HART, GUEST_DEVICE_TREE_OFFSET, GUEST_HEAP_OFFSET, GUEST_STACK_OFFSET,
    GUEST_TEXT_OFFSET, PA2VA_DRAM_OFFSET, PAGE_SIZE, PAGE_TABLE_BASE, PAGE_TABLE_OFFSET_PER_HART,
    STACK_BASE,
};
use crate::memmap::device::plic::{
    CONTEXT_BASE, CONTEXT_CLAIM, CONTEXT_PER_HART, ENABLE_BASE, ENABLE_PER_HART,
};
use crate::memmap::device::Device;
use crate::memmap::{page_table, page_table::PteFlag, DeviceMemmap, MemoryMap};
use crate::trap::supervisor::strap_vector;
use core::arch::asm;
use elf::endian::AnyEndian;
use elf::ElfBytes;
use riscv::register::{satp, sepc, sie, sstatus, stvec};

/// Supervisor start function
/// * Init page tables
/// * Init trap vector
/// * Init stack pointer
#[inline(never)]
pub extern "C" fn sstart(hart_id: usize, dtb_addr: usize) {
    use PteFlag::*;

    // init page tables
    let page_table_start = PAGE_TABLE_BASE + hart_id * PAGE_TABLE_OFFSET_PER_HART;
    let memory_map: [MemoryMap; 6] = [
        // (virtual_memory_range, physical_memory_range, flags),
        // uart
        MemoryMap::new(
            0x1000_0000..0x1000_0100,
            0x1000_0000..0x1000_0100,
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
            0x8020_0000..0x8060_0000,
            0x8020_0000..0x8060_0000,
            &[Dirty, Accessed, Write, Read, Valid],
        ),
        // Device tree
        MemoryMap::new(
            0xbfe0_0000..0xc000_0000,
            0xbfe0_0000..0xc000_0000,
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
            0xffff_ffff_c020_0000..0xffff_ffff_c060_0000,
            0x8020_0000..0x8060_0000,
            &[Dirty, Accessed, Write, Read, Valid],
        ),
    ];
    page_table::generate_page_table(page_table_start, &memory_map, true);

    unsafe {
        // init trap vector
        stvec::write(
            // stvec address must be 4byte aligned.
            trampoline as *const fn() as usize & !0b11,
            stvec::TrapMode::Direct,
        );

        // init stack pointer
        let stack_pointer = STACK_BASE + PA2VA_DRAM_OFFSET;
        let satp_config = (0b1000 << 60) | (page_table_start >> 12);
        asm!(
            "
            mv a0, {hart_id}
            mv a1, {dtb_addr}
            mv sp, {stack_pointer}
            csrw satp, {satp_config}
            sfence.vma
            j {trampoline}
            ",
            hart_id = in(reg) hart_id,
            dtb_addr = in(reg) dtb_addr,
            stack_pointer = in(reg) stack_pointer,
            satp_config = in(reg) satp_config,
            trampoline = sym trampoline
        );
    }

    unreachable!()
}

/// Jump to `smode_setup`
#[inline(never)]
extern "C" fn trampoline(hart_id: usize, dtb_addr: usize) {
    smode_setup(hart_id, dtb_addr);
}

/// Setup for S-mode
/// * parse device tree
/// * Init plic priorities
/// * Set trap vector
/// * Set ppn via setp
/// * Set stack pointer
/// * Jump to `enter_user_mode` via asm j instruction
extern "C" fn smode_setup(hart_id: usize, dtb_addr: usize) {
    unsafe {
        sstatus::clear_sie();
        stvec::write(
            panic_handler as *const fn() as usize + PA2VA_DRAM_OFFSET,
            stvec::TrapMode::Direct,
        );
    }

    // parse device tree
    let device_tree = unsafe {
        match fdt::Fdt::from_ptr(dtb_addr as *const u8) {
            Ok(fdt) => fdt,
            Err(e) => panic!("{}", e),
        }
    };
    let mmap = DeviceMemmap::new(device_tree);

    // set plic priorities
    for plic_num in 1..127 {
        unsafe {
            *((mmap.plic.vaddr() + plic_num * 4) as *mut u32) = 1;
        }
    }

    let mut irq_mask = 0;
    for vio in mmap.virtio.iter().take(4) {
        irq_mask |= 1 << vio.irq();
    }

    // set plic
    unsafe {
        ((mmap.plic.vaddr() + CONTEXT_BASE + CONTEXT_PER_HART * mmap.plic_context) as *mut u32)
            .write_volatile(0);
        ((mmap.plic.vaddr() + ENABLE_BASE + ENABLE_PER_HART * mmap.plic_context) as *mut u32)
            .write_volatile(irq_mask);
        ((mmap.plic.vaddr() + ENABLE_BASE + ENABLE_PER_HART * mmap.plic_context + CONTEXT_CLAIM)
            as *mut u32)
            .write_volatile(0);
    }

    let guest_id = hart_id + 1;
    let guest_base_addr = DRAM_BASE + guest_id * DRAM_SIZE_PAR_HART;
    unsafe {
        // copy dtb to guest space
        let guest_dtb_addr = guest_base_addr + GUEST_DEVICE_TREE_OFFSET + PA2VA_DRAM_OFFSET;
        core::ptr::copy(
            (dtb_addr + PA2VA_DRAM_OFFSET) as *const u8,
            guest_dtb_addr as *mut u8,
            device_tree.total_size(),
        );

        // copy initrd to guest space
        core::ptr::copy(
            mmap.initrd.vaddr() as *const u8,
            (guest_base_addr + GUEST_HEAP_OFFSET + PA2VA_DRAM_OFFSET) as *mut u8,
            mmap.initrd.size(),
        );

        // set sie = 0x222
        sie::set_ssoft();
        sie::set_stimer();
        sie::set_sext();

        // boot page tables
        let page_table_start = guest_base_addr;
        for pt_index in 0..1024 {
            let pt_offset = (page_table_start + pt_index * 8) as *mut usize;
            pt_offset.write_volatile(match pt_index {
                // 0x0000_0000_1xxx_xxxx or 0x0000_0000_1xxx_xxxx
                0 => (page_table_start + PAGE_SIZE) >> 2 | 0x01, // v
                // 0 point to 128 PTE(for 0x0000_0000_1000_0000 -> 0x0000_0000_1000_0000)
                128 => 0x0400_0000 | 0xc7, // d, a, w, r, v
                // 0xffff_fffc_0xxx_xxxx ..= 0xffff_ffff_8xxx_xxxx
                496..=503 => (pt_index - 496) << 28 | 0xcf, // a, d, x, w, r, v
                // 0x0000_0000_8xxx_xxxx or 0xffff_ffff_cxxx_xxxx
                // 0x0000_0000_9xxx_xxxx or 0xffff_ffff_dxxx_xxxx
                2 | 511 => (page_table_start + PAGE_SIZE) >> 2 | 0x01, // v
                // 2 and 511 point to 512 PTE(for 0xffff_ffff_cxxx_xxxx -> 0x0000_0000_8xxx_xxxx)
                512 => 0x2000_0000 | 0xcb, // d, a, x, r, v
                // 2 and 511 point to 640 PTE(for 0xffff_ffff_dxxx_xxxx -> 0x0000_0000_9xxx_xxxx)
                640 => 0x2400_0000 | 0xcf, // d, a, x, w, r, v
                // 2nd level
                513..=1023 => (0x2000_0000 + ((pt_index - 512) << 19)) | 0xc7, // d, a, w, r, v
                _ => 0,
            });
        }

        // satp = Sv39 | 0x9000_0000 >> 12
        satp::set(satp::Mode::Sv39, 0, page_table_start >> 12);

        let stack_pointer = guest_base_addr + GUEST_STACK_OFFSET + PA2VA_DRAM_OFFSET;
        asm!(
            "
            mv a0, {hart_id}
            mv a1, {dtb_addr}
            mv a2, {guest_base_addr}
            mv a3, {guest_id}
            mv a4, {guest_initrd_size}
            mv sp, {stack_pointer_in_umode}
            j {enter_user_mode}
            ",
            hart_id = in(reg) hart_id,
            dtb_addr = in(reg) guest_dtb_addr,
            guest_base_addr = in(reg) guest_base_addr,
            guest_id = in(reg) guest_id,
            guest_initrd_size = in(reg) mmap.initrd.size(),
            stack_pointer_in_umode = in(reg) stack_pointer ,
            enter_user_mode = sym enter_user_mode
        );
    }
}

/// Load elf to guest memory.
///
/// It only load PT_LOAD type segments.
/// Entry address is determined by ... .
///
/// # Arguments
/// * guest_elf - Elf loading guest space.
/// * guest_base_addr - Base address of loading memory space.
fn load_elf(guest_elf: ElfBytes<AnyEndian>, elf_addr: *mut u8, guest_base_addr: usize) -> usize {
    for prog_header in guest_elf
        .segments()
        .expect("failed to get segments from elf")
        .iter()
    {
        const PT_LOAD: u32 = 1;
        if prog_header.p_type == PT_LOAD {
            if prog_header.p_filesz > 0 {
                unsafe {
                    core::ptr::copy(
                        elf_addr.wrapping_add(prog_header.p_offset as usize),
                        (guest_base_addr + prog_header.p_paddr as usize) as *mut u8,
                        prog_header.p_filesz as usize,
                    );
                }
            }
        }
    }

    guest_base_addr
}

/// Prepare to enter U-mode and jump to linux kernel
fn enter_user_mode(
    _hart_id: usize,
    dtb_addr: usize,
    guest_base_addr: usize,
    _guest_id: usize,
    guest_initrd_size: usize,
) {
    unsafe {
        // set sie = 0x222
        sie::set_ssoft();
        sie::set_stimer();
        sie::set_sext();

        // sstatus.SUM = 1, sstatus.SPP = 0
        sstatus::set_sum();
        sstatus::set_spp(sstatus::SPP::User);

        // copy initrd to guest text space(0x9000_0000-) and set initrd entry point to sepc
        let elf_addr = (guest_base_addr + GUEST_HEAP_OFFSET + PA2VA_DRAM_OFFSET) as *mut u8;
        let guest_elf = ElfBytes::<AnyEndian>::minimal_parse(core::slice::from_raw_parts(
            elf_addr,
            guest_initrd_size,
        ))
        .unwrap();
        let entry_point = load_elf(
            guest_elf,
            elf_addr,
            guest_base_addr + GUEST_TEXT_OFFSET + PA2VA_DRAM_OFFSET,
        );
        sepc::write(entry_point);

        // stvec = trap_vector
        stvec::write(
            strap_vector as *const fn() as usize,
            stvec::TrapMode::Direct,
        );

        asm!(
            "
            mv a1, {dtb_addr}

            li ra, 0
            li sp, 0
            li gp, 0
            li tp, 0
            li t0, 0
            li t1, 0
            li t2, 0
            li s0, 0
            li s1, 0
            li a0, 0
            li a2, 0
            li a3, 0
            li a4, 0
            li a5, 0
            li a6, 0
            li a7, 0
            li s2, 0
            li s3, 0
            li s4, 0
            li s5, 0
            li s6, 0
            li s7, 0
            li s8, 0
            li s9, 0
            li s10, 0
            li s11, 0
            li t3, 0
            li t4, 0
            li t5, 0
            li t6, 0
            sret
            ",
            dtb_addr = in(reg) dtb_addr
        );
    }
    unreachable!();
}

/// Panic handler for S-mode
fn panic_handler() {
    panic!("trap from panic macro")
}
