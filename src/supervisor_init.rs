use crate::memmap::Memmap;
use crate::memmap::{
    DRAM_BASE, DRAM_SIZE_PAR_HART, PA2VA_OFFSET, PAGE_TABLE_BASE, PAGE_TABLE_SIZE, STACK_BASE,
    STACK_SIZE_PER_HART,
};
use core::arch::asm;
use riscv::asm::sfence_vma;
use riscv::register::{mtvec, satp, sie, sstatus, stvec};

extern "C" {
    fn trap_vector();
}

/// Supervisor start function
pub fn sstart() {
    // init stack pointer
    let stack_pointer = STACK_BASE + PA2VA_OFFSET;
    unsafe {
        asm!("mv sp, {}", in(reg) stack_pointer);
    }

    let hart_id: usize;
    let dtb_addr: usize;
    unsafe {
        // get an arguments
        asm!("mv {}, a0", out(reg) hart_id);
        asm!("mv {}, a1", out(reg) dtb_addr);
    }

    // init page tables
    let offset_from_dram_base = sstart as *const fn() as usize - DRAM_BASE;
    let offset_from_dram_base_masked = (offset_from_dram_base >> 21) << 19;
    let page_table_start = PAGE_TABLE_BASE + offset_from_dram_base + hart_id * PAGE_TABLE_SIZE;
    for pt_index in 511..1024 {
        let pt_offset = (page_table_start + pt_index * 8) as *mut usize;
        unsafe {
            pt_offset.write_volatile(pt_offset.read_volatile() + offset_from_dram_base_masked);
        }
    }

    unsafe {
        // init trap vector
        stvec::write(trampoline as *const fn() as usize, mtvec::TrapMode::Direct);

        // set satp(Supervisor Address Translation and Protection) register
        satp::set(satp::Mode::Sv39, 0, (page_table_start >> 12) as usize);

        // sfence.vma
        sfence_vma(0, 0);
    }

    // jump to trampoline
    trampoline(hart_id, dtb_addr);

    unreachable!()
}

/// Jump to start
#[inline(never)]
fn trampoline(hart_id: usize, dtb_addr: usize) {
    smode_setup(hart_id, dtb_addr);
}

fn smode_setup(hart_id: usize, dtb_addr: usize) {
    unsafe {
        sstatus::clear_sie();
        stvec::write(
            panic_handler as *const fn() as usize,
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
    let mmap = Memmap::new(device_tree);

    // set plic priorities
    for plic_num in 1..127 {
        unsafe {
            *((mmap.plic.vaddr() + plic_num * 4) as *mut u32) = 1;
        }
    }

    unsafe {
        // set sie = 0x222
        sie::set_ssoft();
        sie::set_stimer();
        sie::set_sext();

        // satp = Sv39 | 0x8000_0000 >> 12
        satp::set(
            satp::Mode::Sv39,
            0,
            (DRAM_BASE + DRAM_SIZE_PAR_HART * hart_id) >> 12,
        );

        let stack_pointer = STACK_BASE + STACK_SIZE_PER_HART * hart_id + PA2VA_OFFSET;
        asm!("mv a0, {dtb_addr}", dtb_addr = in(reg) dtb_addr);
        asm!("mv sp, {stack_pointer_in_umode}", stack_pointer_in_umode = in(reg) stack_pointer);
        asm!("j {enter_user_mode}", enter_user_mode = sym enter_user_mode);
    }
}

fn enter_user_mode(dtb_addr: usize) {
    unsafe {
        // set sie = 0x222
        sie::set_ssoft();
        sie::set_stimer();
        sie::set_sext();

        // sstatus.SUM = 1, sstatus.SPP = 0
        sstatus::set_sum();
        sstatus::set_spp(sstatus::SPP::User);

        stvec::write(trap_vector as *const fn() as usize, stvec::TrapMode::Direct);

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

fn panic_handler() {
    panic!("trap from panic macro")
}
