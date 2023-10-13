//! Constant for memory map.
//!
//! | start       | end         | region    |
//! |-------------|-------------|-----------|
//! | 0x0200_0000 | 0x0210_0000 | QEMU CLINT |
//! | 0x0c00_0000 | 0x0c60_0000 | QEMU PLIC |
//! | 0x1000_0000 | 0x1000_0100 | QEMU UART |
//! | 0x1000_1000 | 0x1000_8000 | QEMU VirtIO |
//! | 0x8000_0000 | 0x8000_xxxx | text data of hikami |
//! | 0x8020_0000 | 0x8020_2000 | hypervisor page table |
//! | 0x8020_4000 | 0x8020_xxxx | hypervisor device tree blob |
//! | 0x8030_0000 | 0x8031_0000 | hypervisor stack |
//! | 0x8040_0000 | 0x8050_0000 | hypervisor heap |
//! | 0x9000_0000 | 0x9000_2000 | hypervisor page table |
//! | 0x9000_2000 | 0x9000_4000 | hypervisor device tree |
//! | 0x9010_0000 | 0x902f_ffff | hypervisor stack |
//! | 0x9030_0000 | 0x93ff_ffff | hypervisor heap |
//! | 0x9400_0000 |     ...     | text data of hikami |

/// Uart addr
pub const UART_ADDR: usize = 0x1000_0000;

/// Base address of dram.
pub const DRAM_BASE: usize = 0x8000_0000;
/// Memory region on dram that be allocated each HARTs.
pub const DRAM_SIZE_PAR_HART: usize = 0x1000_0000;
/// Base address of page table.
pub const PAGE_TABLE_BASE: usize = 0x8020_0000;
/// Size of memory areathat a page can point to.
pub const PAGE_SIZE: usize = 4096;
/// Page table offset for each HART.
pub const PAGE_TABLE_OFFSET_PER_HART: usize = 1024;
/// Base address of stack.
pub const STACK_BASE: usize = 0x8030_0000;
/// Stack size for each HART.
pub const STACK_SIZE_PER_HART: usize = 0x1_0000;
/// Base address of heap.
pub const HEAP_BASE: usize = 0x8040_0000;
/// Heap size.
pub const HEAP_SIZE: usize = 0x10_0000;
/// Offset for converting physical address on dram to virtual address.
pub const PA2VA_DRAM_OFFSET: usize = 0xffff_ffff_4000_0000;
/// Offset for converting physical device address to virtual address.
pub const PA2VA_DEVICE_OFFSET: usize = 0xffff_fffc_0000_0000;

/// loading device tree offset of guest space
pub const GUEST_DEVICE_TREE_OFFSET: usize = 0x2000;
/// Stack offset of guest space
pub const GUEST_STACK_OFFSET: usize = 0x10_0000;
/// Heap offset of guest space
pub const GUEST_HEAP_OFFSET: usize = 0x30_0000;
/// Guest Text secion offset
pub const GUEST_TEXT_OFFSET: usize = 0x0400_0000;
