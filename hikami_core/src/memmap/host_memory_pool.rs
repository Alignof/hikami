//! Host memory that backs guest memory in `iommu` mode.

use super::HostPhysicalAddress;

use alloc::vec::Vec;
use core::ops::Range;

/// Free host DRAM that is used as guest memory.
///
/// It is the host DRAM except for the hypervisor, `/reserved-memory`, and the host device tree.
#[derive(Debug)]
pub struct HostMemoryPool {
    /// Free ranges in ascending order. Each range is aligned to `ALIGN`.
    free_ranges: Vec<Range<usize>>,
}

impl HostMemoryPool {
    /// Alignment of allocation so that guest memory can be mapped with 2 MiB pages.
    const ALIGN: usize = 0x20_0000;

    /// Create a pool from the host DRAM except for `reserved_regions`.
    #[must_use]
    pub fn new(host_dram: &[Range<usize>], reserved_regions: &[Range<usize>]) -> Self {
        let mut free_ranges: Vec<Range<usize>> = host_dram
            .iter()
            .flat_map(|dram| super::subtract_ranges(dram, reserved_regions))
            .filter_map(|range| {
                let start = range.start.next_multiple_of(Self::ALIGN);
                let end = range.end & !(Self::ALIGN - 1);
                (start < end).then_some(start..end)
            })
            .collect();
        free_ranges.sort_by_key(|range| range.start);

        HostMemoryPool { free_ranges }
    }

    /// Return free ranges.
    #[must_use]
    pub fn free_ranges(&self) -> &[Range<usize>] {
        &self.free_ranges
    }

    /// Allocate `size` bytes (rounded up to `ALIGN`).
    ///
    /// The allocated memory may be split into several ranges.
    ///
    /// # Panics
    /// Panics if the pool does not have enough memory.
    pub fn alloc(&mut self, size: usize) -> Vec<Range<HostPhysicalAddress>> {
        let mut remaining = size.next_multiple_of(Self::ALIGN);
        let mut allocated = Vec::new();

        while remaining > 0 {
            let free = self
                .free_ranges
                .first_mut()
                .expect("host memory for guests is exhausted");
            let len = remaining.min(free.end - free.start);
            allocated.push(HostPhysicalAddress(free.start)..HostPhysicalAddress(free.start + len));
            free.start += len;
            remaining -= len;

            if free.start == free.end {
                self.free_ranges.remove(0);
            }
        }

        allocated
    }
}
