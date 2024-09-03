//! Remote fence implementation for `RustSBI`.

use rustsbi::{HartMask, SbiRet};

/// Remote fence implementation.
/// ref: [https://docs.rs/rustsbi/0.4.0-alpha.3/rustsbi/trait.Fence.html](https://docs.rs/rustsbi/0.4.0-alpha.3/rustsbi/trait.Fence.html)
struct RemoteFence;

impl RemoteFence {
    // Required methods
    fn remote_fence_i(&self, hart_mask: HartMask) -> SbiRet {}
    fn remote_sfence_vma(&self, hart_mask: HartMask, start_addr: usize, size: usize) -> SbiRet {}
    fn remote_sfence_vma_asid(
        &self,
        hart_mask: HartMask,
        start_addr: usize,
        size: usize,
        asid: usize,
    ) -> SbiRet {
    }
}
