use core::arch::asm;

/// Guest context
#[allow(dead_code)]
#[repr(C)]
#[derive(Debug, Default)]
pub struct Context {
    /// Registers
    pub xreg: [u64; 32],
    /// Program counter
    pub sstatus: u32,
    /// Value of sstatus
    pub sepc: usize,
}

impl Context {
    /// Load context data to registers.
    ///
    /// # Safety
    /// If `Context.addr` is valid address.
    #[inline(always)]
    #[allow(clippy::inline_always)]
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
    ///
    /// # Safety
    /// If `Context.addr` is valid address.
    #[inline(always)]
    #[allow(clippy::inline_always)]
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

                // restore sp
                csrr sp, sscratch
                ",
                context_addr = in(reg) self,
            );
        }
    }

    pub fn set_xreg(&mut self, index: usize, value: u64) {
        self.xreg[index] = value;
    }

    pub fn set_sepc(&mut self, value: usize) {
        self.sepc = value;
    }
}
