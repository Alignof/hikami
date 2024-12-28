//! Constant for memory map.
//!
//! # Host physical address
//! See `memory.x`.
//! | start         | end           | region              |
//! |---------------|---------------|---------------------|
//! | `0x8000_0000` | `0x8000_XXXX` | text data of hikami |
//!
//! # Guest physical address
//! | start         | end           | region                 |
//! |---------------|---------------|------------------------|
//! | `0xXXXX_XXXX` | `0xXXXX_XXXX` | device identity map    |
//! |               |               |                        |
//! | `0x8000_0000` | `0x8000_2000` | device tree of guest 1 |
//! | `0x9000_0000` | `0xa000_0000` | text data of guest 1   |

/// Max number of HART
pub const MAX_HART_NUM: usize = 8;
/// Base address of dram.
pub const DRAM_BASE: usize = 0x8000_0000;
/// Stack size for each HART.
pub const STACK_SIZE_PER_HART: usize = 0x1_0000;

pub mod device {
    //! Device memory map
    //! TODO?: parse device tree in `machine_init.rs`

    use crate::memmap::HostPhysicalAddress;

    /// CLINT address
    /// For trap `SupervisorSoftware` interrupt
    pub const CLINT_ADDR: HostPhysicalAddress = HostPhysicalAddress(0x200_0000);

    /// mtimecmp CSRs address
    pub const MTIMECMP_ADDR: HostPhysicalAddress = HostPhysicalAddress(0x200_4000);
}

pub mod guest_memory {
    //! Guest memory region on Guest Physical Address

    use crate::memmap::GuestPhysicalAddress;

    /// Dram base address
    pub const DRAM_BASE: GuestPhysicalAddress = GuestPhysicalAddress(super::DRAM_BASE);
    /// Dram memory space per HART.
    pub const DRAM_SIZE_PER_GUEST: usize = 256 * 1024 * 1024; // 256 MB = 0x1000_0000
    /// Guest DTB space size
    pub const GUEST_DTB_SIZE_PER_HART: usize = 0x2000;
}
