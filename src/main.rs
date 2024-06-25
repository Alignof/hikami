#![no_main]
#![no_std]

extern crate alloc;
mod guest;
mod h_extension;
mod hypervisor_init;
mod machine_init;
mod memmap;
mod supervisor_init;
mod trap;
mod util;

use crate::machine_init::mstart;
use crate::memmap::constant::{DRAM_BASE, HEAP_BASE, HEAP_SIZE, STACK_BASE, STACK_SIZE_PER_HART};
use core::arch::asm;
use core::panic::PanicInfo;
use riscv_rt::entry;
use spin::Mutex;
use wild_screen_alloc::WildScreenAlloc;

/// Panic handler
#[panic_handler]
pub fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {
        unsafe {
            asm!("nop");
        }
    }
}

/// Global data for hypervisor.
///
/// FIXME: Rename me!
#[derive(Debug)]
pub struct HypervisorData {
    pub context: Context,
}

impl Default for HypervisorData {
    fn default() -> Self {
        HypervisorData {
            context: Context::default(),
        }
    }
}

/// Guest context
#[repr(packed)]
#[derive(Debug)]
pub struct Context {
    /// Registers
    registers: [u32; 32],
    /// Program counter
    pc: u32,
    /// Value of sstatus
    xstatus: usize,
}

impl Context {
    /// Load context data to registers.
    #[inline(always)]
    pub unsafe fn load(&self) {
        unsafe {
            asm!(
                "
                fence.i
                csrw sscratch, sp
                mv sp, {context_addr}

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
                ld a1, 11*8(sp)
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
                csrr sp, sscratch
                ",
                context_addr = in(reg) self,
            );
        }
    }

    /// Store context data to registers.
    #[inline(always)]
    pub unsafe fn store(&mut self) {
        unsafe {
            asm!(
                "
                fence.i
                csrw sscratch, sp
                mv sp, {context_addr}

                // save sstatus
                csrr t0, sstatus
                sd t0, 32*8(sp)

                // save pc
                csrr t1, sepc
                sd t1, 33*8(sp)

                // save registers
                sd ra, 1*8(sp)
                sd gp, 3*8(sp)
                sd tp, 4*8(sp)
                sd t0, 5*8(sp)
                sd t1, 6*8(sp)
                sd t2, 7*8(sp)
                sd s0, 8*8(sp)
                sd s1, 9*8(sp)
                sd a0, 10*8(sp)
                sd a1, 11*8(sp)
                sd a2, 12*8(sp)
                sd a3, 13*8(sp)
                sd a4, 14*8(sp)
                sd a5, 15*8(sp)
                sd a6, 16*8(sp)
                sd a7, 17*8(sp)
                sd s2, 18*8(sp)
                sd s3, 19*8(sp)
                sd s4, 20*8(sp)
                sd s5, 21*8(sp)
                sd s6, 22*8(sp)
                sd s7, 23*8(sp)
                sd s8, 24*8(sp)
                sd s9, 25*8(sp)
                sd s10, 26*8(sp)
                sd s11, 27*8(sp)
                sd t3, 28*8(sp)
                sd t4, 29*8(sp)
                sd t5, 30*8(sp)
                sd t6, 31*8(sp)

                // save stack pointer
                csrr t0, sscratch
                sd t0, 2*8(sp)
                ",
                context_addr = in(reg) self,
            );
        }
    }
}

impl Default for Context {
    fn default() -> Self {
        Context {
            registers: [0u32; 32],
            pc: 0u32,
            xstatus: 0usize,
        }
    }
}

#[global_allocator]
static mut ALLOCATOR: WildScreenAlloc = WildScreenAlloc::empty();

static mut HYPERVISOR_DATA: Mutex<HypervisorData> = OnceCell::new();

/// Entry function. `__risc_v_rt__main` is alias of `__init` function in machine_init.rs.
/// * set stack pointer
/// * init mtvec and stvec
/// * jump to mstart
#[entry]
fn _start(hart_id: usize, dtb_addr: usize) -> ! {
    unsafe {
        // Initialize global allocator
        ALLOCATOR.init(HEAP_BASE, HEAP_SIZE);
        // Initialize global hypervisor data
        HYPERVISOR_DATA
            .set(Mutex::new(HypervisorData::default()))
            .expect("hypervisor global data initialization failed");
    }

    unsafe {
        // set stack pointer
        asm!(
            "
            mv a0, {hart_id}
            mv a1, {dtb_addr}
            mv t1, {stack_size_per_hart}
            mul t0, a0, t1
            mv sp, {stack_base}
            add sp, sp, t0
            csrw mtvec, {DRAM_BASE}
            csrw stvec, {DRAM_BASE}
            j {mstart}
            ",
            hart_id = in(reg) hart_id,
            dtb_addr = in(reg) dtb_addr,
            stack_size_per_hart = in(reg) STACK_SIZE_PER_HART,
            stack_base = in(reg) STACK_BASE,
            DRAM_BASE = in(reg) DRAM_BASE,
            mstart = sym mstart,
        );
    }

    unreachable!();
}
