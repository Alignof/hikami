//! Constant for memory map.

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
