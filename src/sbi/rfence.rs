//! Remote fence implementation for `RustSBI`.

use core::arch::asm;
use rustsbi::{HartMask, SbiRet};

use crate::memmap::constant::MAX_HART_NUM;
use crate::memmap::page_table::constants::PAGE_SIZE;

/// Remote fence implementation.
/// ref: [https://docs.rs/rustsbi/0.4.0-alpha.3/rustsbi/trait.Fence.html](https://docs.rs/rustsbi/0.4.0-alpha.3/rustsbi/trait.Fence.html)
pub struct RemoteFence;

impl rustsbi::Fence for RemoteFence {
    // Required methods
    fn remote_fence_i(&self, hart_mask: HartMask) -> SbiRet {
        // current hart must be 0.
        for hart_id in 1..MAX_HART_NUM {
            debug_assert!(!hart_mask.has_bit(hart_id));
        }

        if hart_mask.has_bit(0) {
            unsafe { asm!("fence.i") }
        }

        SbiRet::success(0)
    }

    fn remote_sfence_vma(&self, hart_mask: HartMask, start_addr: usize, size: usize) -> SbiRet {
        // current hart must be 0.
        for hart_id in 1..MAX_HART_NUM {
            debug_assert!(!hart_mask.has_bit(hart_id));
        }

        for addr in (start_addr..start_addr + size).step_by(PAGE_SIZE) {
            unsafe { asm!("sfence.vma {vaddr}, x0", vaddr = in(reg) addr) }
        }

        SbiRet::success(0)
    }

    fn remote_sfence_vma_asid(
        &self,
        hart_mask: HartMask,
        start_addr: usize,
        size: usize,
        asid: usize,
    ) -> SbiRet {
        // current hart must be 0.
        for hart_id in 1..MAX_HART_NUM {
            debug_assert!(!hart_mask.has_bit(hart_id));
        }

        for addr in (start_addr..start_addr + size).step_by(PAGE_SIZE) {
            unsafe { asm!("sfence.vma {vaddr}, {asid}", vaddr = in(reg) addr, asid = in(reg) asid) }
        }

        SbiRet::success(0)
    }
}
