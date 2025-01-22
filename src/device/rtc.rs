//! RTC: Real Time Clock.

use super::{MmioDevice, PTE_FLAGS_FOR_DEVICE};
use crate::memmap::{GuestPhysicalAddress, HostPhysicalAddress, MemoryMap};
use fdt::Fdt;

/// RTC: Real Time Clock.
/// An electronic device that measures the passage of time.
#[derive(Debug)]
pub struct Rtc {
    /// Base address of memory map.
    base_addr: HostPhysicalAddress,
    /// Memory map size.
    size: usize,
}

impl MmioDevice for Rtc {
    fn new(device_tree: &Fdt, compatibles: &[&str]) -> Self {
        let region = device_tree
            .find_compatible(compatibles)
            .unwrap()
            .reg()
            .unwrap()
            .next()
            .unwrap();

        Rtc {
            base_addr: HostPhysicalAddress(region.starting_address as usize),
            size: region.size.unwrap(),
        }
    }

    fn size(&self) -> usize {
        self.size
    }

    fn paddr(&self) -> HostPhysicalAddress {
        self.base_addr
    }

    fn memmap(&self) -> MemoryMap {
        let vaddr = GuestPhysicalAddress(self.paddr().raw());
        MemoryMap::new(
            vaddr..vaddr + self.size(),
            self.paddr()..self.paddr() + self.size(),
            &PTE_FLAGS_FOR_DEVICE,
        )
    }
}
