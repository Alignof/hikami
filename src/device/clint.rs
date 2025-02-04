//! CLINT: *C*ore *L*ocal *Int*errupt

use super::{MmioDevice, PTE_FLAGS_FOR_DEVICE};
use crate::memmap::{GuestPhysicalAddress, HostPhysicalAddress, MemoryMap};
use fdt::Fdt;

mod register {
    //! Ref: [https://chromitem-soc.readthedocs.io/en/latest/clint.html](https://chromitem-soc.readthedocs.io/en/latest/clint.html)
    //!
    //! | Register-Name | Offset(hex) | Size(Bits) | Reset(hex) | Description                                                        |
    //! | ------------- | ----------- | ---------- | ---------- | -----------                                                        |
    //! | msip          | 0x0         | 32         | 0x0        | This register generates machine mode software interrupts when set. |
    //! | mtimecmp      | 0x4000      | 64         | 0x0        | This register holds the compare value for the timer.               |
    //! | mtime         | 0xBFF8      | 64         | 0x0        | Provides the current timer value.                                  |

    /// Offset of `MISP` register
    pub const MSIP_OFFSET: usize = 0x0;
    /// Offset of `MTIMECMP` register
    pub const MTIMECMP_OFFSET: usize = 0x4000;
    /// Offset of `MTIME` register
    pub const MTIME_OFFSET: usize = 0xbff8;
}

#[allow(clippy::doc_markdown)]
/// CLINT: Core Local INTerrupt
/// Local interrupt controller
#[derive(Debug)]
pub struct Clint {
    /// Base address of memory map.
    base_addr: HostPhysicalAddress,
    /// Memory map size.
    size: usize,
}

impl MmioDevice for Clint {
    fn try_new(device_tree: &Fdt, compatibles: &[&str]) -> Option<Self> {
        let region = device_tree
            .find_compatible(compatibles)?
            .reg()
            .unwrap()
            .next()
            .unwrap();

        Some(Clint {
            base_addr: HostPhysicalAddress(region.starting_address as usize),
            size: region.size.unwrap(),
        })
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
