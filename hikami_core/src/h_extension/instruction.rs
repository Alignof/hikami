//! Utility for H extension instructions.

use crate::memmap::GuestVirtualAddress;

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

/// Read a halfword from guest memory as an instruction fetch (HLVX.HU).
///
/// `gva` is translated by two-stage address translation with the privilege mode in
/// `hstatus.SPVP`, which is set to the guest privilege mode when a trap is taken from VS/VU-mode.
///
/// # Safety
/// `gva` must be mapped and executable in the guest. Otherwise HLVX.HU raises an exception
/// in HS-mode.
#[inline(always)]
#[allow(clippy::inline_always)]
pub unsafe fn hlvx_hu(gva: GuestVirtualAddress) -> u16 {
    let value: u16;
    unsafe {
        asm!(
            ".option push",
            ".option arch, +h",
            "hlvx.hu {value}, ({addr})",
            ".option pop",
            value = out(reg) value,
            addr = in(reg) gva.0,
            options(nostack, readonly),
        );
    }
    value
}

/// Fetch a guest instruction at `gva` (e.g. `sepc` of a trap from the guest).
///
/// It reads a 16-bit parcel first and the next one only for a 32-bit instruction,
/// so it works even if the instruction crosses a page boundary.
///
/// # Safety
/// See `hlvx_hu`. The instruction at `gva` has been fetched by the guest when the trap
/// was caused by the instruction itself (not by its fetch).
#[must_use]
pub unsafe fn fetch_guest_instruction(gva: GuestVirtualAddress) -> usize {
    let low = usize::from(unsafe { hlvx_hu(gva) });

    // 32-bit instruction
    if low & 0b11 == 0b11 {
        let high = usize::from(unsafe { hlvx_hu(GuestVirtualAddress(gva.0 + 2)) });
        (high << 16) | low
    // 16-bit instruction
    } else {
        low
    }
}
