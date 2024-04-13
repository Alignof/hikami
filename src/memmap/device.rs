//! A module about device on memory map.  
//! This module holds each devices implementation.

pub mod initrd;
pub mod plic;
pub mod uart;
pub mod virtio;

use super::MemoryMap;
use fdt::Fdt;

/// A struct that implement Device trait **must** has `base_addr` and size member.
pub trait Device {
    /// Create self instance.
    /// * `device_tree` - struct Fdt
    /// * `node_path` - node path in fdt
    fn new(device_tree: &Fdt, node_path: &str) -> Self;
    /// Return size of memory region.
    fn size(&self) -> usize;
    /// Return address of physical memory
    fn paddr(&self) -> usize;
    /// Return address of virtual memory
    fn vaddr(&self) -> usize;
    /// Return memory map between virtual to physical
    fn memmap(&self) -> MemoryMap;
    /// Return memory map between physical to physical
    fn identity_memmap(&self) -> MemoryMap;
}
