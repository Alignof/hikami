//! Constant for memory map.
//!
//! | start       | end         | region                     |
//! |-------------|-------------|----------------------------|
//! | 0x0200_0000 | 0x0210_0000 | QEMU CLINT                 |
//! | 0x0c00_0000 | 0x0c60_0000 | QEMU PLIC                  |
//! | 0x1000_0000 | 0x1000_0100 | QEMU UART                  |
//! | 0x1000_1000 | 0x1000_8000 | QEMU VirtIO                |
//! |             |             |                            |
//! | 0x8000_0000 | 0x8020_0000 | text data of hikami        |
//! | 0x8020_0000 |     ...     | bottom of stack            |
//! |     ...     | 0x8080_0000 | machine stack              |
//! |             |             |                            |
//! | 0x8100_0000 | 0x8100_2000 | G-stage root page table    |
//! | 0x8100_2000 | 0x8100_4000 | device tree blob for guest |
//! | 0x8200_0000 |     ...     | hypervisor heap            |
//! |     ...     | 0xXXXX_XXXX | hypervisor stack           |

/// Max number of HART
pub const MAX_HART_NUM: usize = 8;
/// Base address of dram.
pub const DRAM_BASE: usize = 0x8000_0000;
/// Dram memory space per HART.
pub const GUEST_DRAM_SIZE: usize = 256 * 1024 * 1024; // 256 MB
/// Stack size for each HART.
pub const STACK_SIZE_PER_HART: usize = 0x1_0000;
/// Offset for converting physical device address to virtual address.
pub const PA2VA_DEVICE_OFFSET: usize = 0xffff_fffc_0000_0000;

pub mod device {
    //! Device memory map

    /// Uart address
    /// For println macro.
    pub const UART_ADDR: usize = 0x1000_0000;

    /// CLINT address
    /// For trap `SupervisorSoftware` interrupt
    pub const CLINT_ADDR: usize = 0x200_0000;

    /// mtimecmp CSRs address
    pub const MTIMECMP_ADDR: usize = 0x200_4000;
}

pub mod machine {
    //! Machine memory region (`0x8020_0000` - `0x8080_0000`)

    /// Base address of machine stack.
    pub const STACK_BASE: usize = 0x8080_0000;
}

pub mod hypervisor {
    //! Hypervisor memory region (`0x8120_0000` - `0x8300_0000`)

    /// Base address of hypervisor region.
    pub const BASE_ADDR: usize = 0x8100_0000;

    /// Base address of page table.
    pub const PAGE_TABLE_OFFSET: usize = 0x0;
    /// Page table offset for each HART.
    pub const PAGE_TABLE_OFFSET_PER_HART: usize = 1024;
    /// Base address of device tree blob for guest image
    pub const GUEST_DEVICE_TREE_OFFSET: usize = 0x2000;
    /// Base address of heap.
    pub const HEAP_OFFSET: usize = 0x2000_0000;
    /// Heap size.
    pub const HEAP_SIZE: usize = 0x1000_0000;
}
