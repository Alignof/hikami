use core::arch::asm;

#[inline(always)]
#[allow(clippy::inline_always)]
pub fn hfence_gvma_all() {
    unsafe {
        asm!("hfence.gvma x0, x0");
    }
}
