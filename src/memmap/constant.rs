//! Constant for memory map.
//!
//! # Guest physical address
//! | start         | end           | region                   |
//! |---------------|---------------|--------------------------|
//! | `0xXXXX_XXXX` | `0xXXXX_XXXX` | device identity map      |
//! |               |               |                          |
//! | `0x8000_0000` | `0x8000_2000` | device tree of guest 1   |
//! | `0x9000_0000` | `0xa000_0000` | Memory region of guest 1 |
//! | `0x9fff_d000` | `0xa000_0000` | Device tree of guest 1   |

/// Max number of HART
pub const MAX_HART_NUM: usize = 8;
/// Base address of dram.
pub const DRAM_BASE: usize = 0x8000_0000;
/// Stack size for each HART.
pub const STACK_SIZE_PER_HART: usize = 0x1_0000;

pub mod guest_memory {
    //! Guest memory region on Guest Physical Address

    use crate::memmap::GuestPhysicalAddress;

    /// Dram base address in guest memory
    ///
    /// It starts from as high as `DRAM_SIZE_PER_GUEST` to distinguish from HPA.
    pub const DRAM_BASE: GuestPhysicalAddress = GuestPhysicalAddress(super::DRAM_BASE);
    /// Dram memory space per HART.
    pub const DRAM_SIZE_PER_GUEST: usize = 256 * 1024 * 1024; // 256 MB = 0x1000_0000
    /// Guest DTB space size
    pub const GUEST_DTB_REGION_SIZE: usize = 0x2000;
}
