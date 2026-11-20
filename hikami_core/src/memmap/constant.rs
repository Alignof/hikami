//! Constants for memory map.
//!
//! Board-specific memory layout is given by `Board` and the device trees.

/// Max number of HART
pub const MAX_HART_NUM: usize = 8;

pub mod guest_memory {
    //! Guest memory region on Guest Physical Address

    /// Guest DTB space size
    pub const GUEST_DTB_REGION_SIZE: usize = 0x26000;
}
