use crate::memmap::constant::STACK_BASE;
use core::arch::asm;

/// Guest context on memory
#[allow(dead_code)]
#[repr(C)]
pub struct ContextData {
    /// Registers
    pub xreg: [u64; 32],
    /// Value of sstatus
    pub sstatus: usize,
    /// Program counter
    pub sepc: usize,
}

/// Guest context
#[derive(Debug, Copy, Clone)]
pub struct Context {
    address: usize,
}

impl Default for Context {
    fn default() -> Self {
        Context {
            address: STACK_BASE,
        }
    }
}

impl Context {
    /// Get `ContextData` from raw address.
    #[allow(clippy::mut_from_ref)]
    fn get_context(&self) -> &mut ContextData {
        unsafe {
            (self.address as *mut ContextData)
                .as_mut()
                .expect("address of ContextData is invalid")
        }
    }

    pub fn xreg(&self, index: usize) -> u64 {
        self.get_context().xreg[index]
    }

    pub fn set_xreg(&mut self, index: usize, value: u64) {
        self.get_context().xreg[index] = value;
    }

    pub fn sepc(&self) -> usize {
        self.get_context().sepc
    }

    pub fn set_sepc(&mut self, value: usize) {
        self.get_context().sepc = value;
    }
}

/// Load context data from memory.
///
/// # Safety
/// If `Context.addr` is valid address.
///
/// # TODO
/// replace stringify macro to const when `asm_const` is stabled.
#[inline(always)]
#[allow(clippy::inline_always)]
pub unsafe fn load() {
    unsafe {
        asm!(
            "
                fence.i
                li sp, 0x80200000 // STATIC_BASE + CONTEXT_OFFSET

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
                ",
        );
    }
}

/// Store context data to memory.
///
/// # Safety
/// If `Context.addr` is valid address.
///
/// # TODO
/// replace stringify macro to const when `asm_const` is stabled.
#[inline(always)]
#[allow(clippy::inline_always)]
pub unsafe fn store() {
    unsafe {
        asm!(
            "
                fence.i
                li sp, 0x80200000 // STATIC_BASE + CONTEXT_OFFSET
                
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

                // save sstatus
                csrr t0, sstatus
                sd t0, 32*8(sp)

                // save pc
                csrr t1, sepc
                sd t1, 33*8(sp)
                ",
        );
    }
}
