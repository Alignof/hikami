//! See `memmap/constant` module for specefic memmory map.

pub mod constant;
pub mod page_table;

use crate::memmap::page_table::PteFlag;

use alloc::vec::Vec;
use core::ops::Range;
use fdt::{Fdt, standard_nodes::MemoryRegion};

/// Utility for `Range<Address>`
trait AddressRangeUtil {
    /// Return length of range.
    fn len(&self) -> usize;
}

/// Guest Virtual Address
#[derive(Default, Debug, Copy, Clone)]
pub struct GuestVirtualAddress(pub usize);

/// Guest Physical Address
#[derive(Default, Debug, Copy, Clone, PartialEq, PartialOrd, Ord, Eq)]
pub struct GuestPhysicalAddress(pub usize);

impl GuestPhysicalAddress {
    /// Convert to usize.
    #[must_use]
    pub fn raw(self) -> usize {
        self.0
    }
}

impl core::ops::Add<usize> for GuestPhysicalAddress {
    type Output = GuestPhysicalAddress;
    fn add(self, other: usize) -> Self::Output {
        GuestPhysicalAddress(self.0 + other)
    }
}

impl core::ops::Sub<usize> for GuestPhysicalAddress {
    type Output = GuestPhysicalAddress;
    fn sub(self, other: usize) -> Self::Output {
        GuestPhysicalAddress(self.0 - other)
    }
}

impl core::ops::Rem<usize> for GuestPhysicalAddress {
    type Output = usize;
    fn rem(self, other: usize) -> Self::Output {
        self.0 % other
    }
}

impl AddressRangeUtil for Range<GuestPhysicalAddress> {
    fn len(&self) -> usize {
        self.end.raw() - self.start.raw()
    }
}

/// Host Physical Address
#[derive(Default, Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct HostPhysicalAddress(pub usize);

impl HostPhysicalAddress {
    /// Convert to usize.
    #[must_use]
    pub fn raw(self) -> usize {
        self.0
    }
}

impl core::ops::Add<usize> for HostPhysicalAddress {
    type Output = HostPhysicalAddress;
    fn add(self, other: usize) -> Self::Output {
        HostPhysicalAddress(self.0 + other)
    }
}

impl core::ops::Sub<usize> for HostPhysicalAddress {
    type Output = HostPhysicalAddress;
    fn sub(self, other: usize) -> Self::Output {
        HostPhysicalAddress(self.0 - other)
    }
}

impl core::ops::Rem<usize> for HostPhysicalAddress {
    type Output = usize;
    fn rem(self, other: usize) -> Self::Output {
        self.0 % other
    }
}

impl AddressRangeUtil for Range<HostPhysicalAddress> {
    fn len(&self) -> usize {
        self.end.raw() - self.start.raw()
    }
}

/// Struct for represent memory regtion.
#[derive(Debug, Clone)]
pub struct MemoryMap {
    /// Guest physical address
    virt: Range<GuestPhysicalAddress>,
    /// Host physical address
    pub phys: Range<HostPhysicalAddress>,
    /// Page table entry flags
    flags: u8,
}

impl MemoryMap {
    /// Create new `MemoryMap`.
    ///
    /// `flags` is mapped to bitmap.
    #[must_use]
    pub fn new(
        virt: Range<GuestPhysicalAddress>,
        phys: Range<HostPhysicalAddress>,
        flags: &[PteFlag],
    ) -> Self {
        Self {
            virt,
            phys,
            flags: flags.iter().fold(0, |pte_f, f| pte_f | *f as u8),
        }
    }
    /// Return flags as raw u8.
    pub fn flags(&self) -> u8 {
        self.flags
    }

    /// Return flags as an array of PteFlag.
    pub fn flags_as_array(&self) -> Vec<PteFlag> {
        let mut result = Vec::new();
        if self.flags & PteFlag::Valid as u8 != 0 {
            result.push(PteFlag::Valid);
        }
        if self.flags & PteFlag::Read as u8 != 0 {
            result.push(PteFlag::Read);
        }
        if self.flags & PteFlag::Write as u8 != 0 {
            result.push(PteFlag::Write);
        }
        if self.flags & PteFlag::Exec as u8 != 0 {
            result.push(PteFlag::Exec);
        }
        if self.flags & PteFlag::User as u8 != 0 {
            result.push(PteFlag::User);
        }
        if self.flags & PteFlag::Global as u8 != 0 {
            result.push(PteFlag::Global);
        }
        if self.flags & PteFlag::Accessed as u8 != 0 {
            result.push(PteFlag::Accessed);
        }
        if self.flags & PteFlag::Dirty as u8 != 0 {
            result.push(PteFlag::Dirty);
        }
        result
    }
}

impl From<fdt::standard_nodes::MemoryRegion> for MemoryMap {
    fn from(region: fdt::standard_nodes::MemoryRegion) -> Self {
        let size = region.size.unwrap();
        let virt_start = GuestPhysicalAddress(region.starting_address as usize);
        let phys_start = HostPhysicalAddress(region.starting_address as usize);
        MemoryMap::new(
            virt_start..virt_start + size,
            phys_start..phys_start + size,
            &[
                PteFlag::Dirty,
                PteFlag::Accessed,
                PteFlag::Write,
                PteFlag::Read,
                PteFlag::User,
                PteFlag::Valid,
            ],
        )
    }
}

/// Return the parts of `range` that do not overlap any of `holes`, in ascending order.
#[must_use]
pub fn subtract_ranges(range: &Range<usize>, holes: &[Range<usize>]) -> Vec<Range<usize>> {
    let mut holes: Vec<Range<usize>> = holes
        .iter()
        .filter(|hole| hole.start < range.end && range.start < hole.end)
        .cloned()
        .collect();
    holes.sort_by_key(|hole| hole.start);

    let mut result = Vec::new();
    let mut cursor = range.start;
    for hole in holes {
        if cursor < hole.start {
            result.push(cursor..hole.start);
        }
        cursor = cursor.max(hole.end);
    }
    if cursor < range.end {
        result.push(cursor..range.end);
    }
    result
}

/// Return the regions of all `memory` nodes in the device tree.
#[must_use]
pub fn memory_regions(device_tree: &Fdt) -> Vec<Range<usize>> {
    device_tree
        .all_nodes()
        .filter(|node| {
            node.property("device_type")
                .and_then(fdt::node::NodeProperty::as_str)
                == Some("memory")
        })
        .filter_map(fdt::node::FdtNode::reg)
        .flatten()
        .map(region_to_range)
        .collect()
}

/// Return the regions of all children of `/reserved-memory` in the device tree.
#[must_use]
pub fn reserved_memory_regions(device_tree: &Fdt) -> Vec<Range<usize>> {
    device_tree
        .find_node("/reserved-memory")
        .into_iter()
        .flat_map(fdt::node::FdtNode::children)
        .filter_map(fdt::node::FdtNode::reg)
        .flatten()
        .map(region_to_range)
        .collect()
}

/// Convert `MemoryRegion` of the device tree to an address range.
fn region_to_range(region: MemoryRegion) -> Range<usize> {
    let start = region.starting_address as usize;
    start..start + region.size.unwrap_or(0)
}
