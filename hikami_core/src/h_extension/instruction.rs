//! Utility for H extension instructions.

use core::arch::asm;

/// Hypervisor memory management fence for all virtual machines and guest physical addresses.
#[inline(always)]
#[allow(clippy::inline_always)]
pub fn hfence_gvma_all() {
    unsafe {
        asm!(
            ".option push",
            ".option arch, +h",
            "hfence.gvma x0, x0",
            ".option pop",
        );
    }
}
