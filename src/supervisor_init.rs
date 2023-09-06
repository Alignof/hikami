use crate::memmap::Memmap;
use crate::memmap::{
    DRAM_BASE, DRAM_SIZE_PAR_HART, PA2VA_DRAM_OFFSET, PAGE_SIZE, PAGE_TABLE_BASE,
    PAGE_TABLE_OFFSET_PER_HART, STACK_BASE, STACK_SIZE_PER_HART,
};
use crate::trap_vector;
use core::arch::asm;
use riscv::register::{satp, sie, sstatus, stvec};

/// Supervisor start function
pub fn sstart() {
    let hart_id: usize;
    let dtb_addr: usize;
    unsafe {
        // get an arguments
        asm!("mv {}, a0", out(reg) hart_id);
        asm!("mv {}, a1", out(reg) dtb_addr);
    }

    // init page tables
    let page_table_start = PAGE_TABLE_BASE + hart_id * PAGE_TABLE_OFFSET_PER_HART;
    for pt_index in 0..1024 {
        let pt_offset = (page_table_start + pt_index * 8) as *mut usize;
        unsafe {
            pt_offset.write_volatile(match pt_index {
                // 0x0000_0000_8xxx_xxxx or 0xffff_ffff_cxxx_xxxx
                2 | 511 => (PAGE_TABLE_BASE + PAGE_SIZE) >> 2 | 0x01, // v
                // 2 and 511 point to 512 PTE
                512 => 0x2000_0000 | 0xcb, // d, a, x, r, v
                // 2nd level
                513..=1023 => (0x2000_0000 + ((pt_index - 512) << 19)) | 0xc7, // d, a, w, r, v
                _ => 0,
            });
        }
    }

    unsafe {
        // init trap vector
        stvec::write(
            // stvec address must be 4byte aligned.
            trampoline as *const fn() as usize & !0b11,
            //trampoline as *const fn() as usize + PA2VA_DRAM_OFFSET & !0b11,
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

/// Jump to start
#[inline(never)]
fn trampoline(hart_id: usize, dtb_addr: usize) {
    smode_setup(hart_id, dtb_addr);
}

fn smode_setup(hart_id: usize, dtb_addr: usize) {
    unsafe {
        sstatus::clear_sie();
        stvec::write(
            panic_handler as *const fn() as usize + PA2VA_DRAM_OFFSET,
            stvec::TrapMode::Direct,
        );
    }

    // parse device tree
    let device_tree = unsafe {
        match fdt::Fdt::from_ptr((dtb_addr + PA2VA_DRAM_OFFSET) as *const u8) {
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

        let stack_pointer = STACK_BASE + STACK_SIZE_PER_HART * hart_id + PA2VA_DRAM_OFFSET;
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
