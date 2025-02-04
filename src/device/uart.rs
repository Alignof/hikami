//! UART: Universal Asynchronous Receiver-Transmitter

use super::{MmioDevice, PTE_FLAGS_FOR_DEVICE};
use crate::memmap::{GuestPhysicalAddress, HostPhysicalAddress, MemoryMap};

use core::cell::OnceCell;
use fdt::Fdt;
use spin::Mutex;

mod register {
    //! Ref: [http://byterunner.com/16550.html](http://byterunner.com/16550.html)

    /// LSR register offset.
    pub const LSR_OFFSET: usize = 3;
}

/// Uart address for `UartWriter`.
static UART_ADDR: Mutex<OnceCell<HostPhysicalAddress>> = Mutex::new(OnceCell::new());

/// UART: Universal asynchronous receiver-transmitter
#[derive(Debug)]
pub struct Uart {
    /// Base address of memory map.
    base_addr: HostPhysicalAddress,
    /// Memory map size.
    size: usize,
}

impl Uart {
    /// Return address of LSR register.
    pub fn lsr_addr(&self) -> HostPhysicalAddress {
        self.base_addr + register::LSR_OFFSET
    }
}

impl MmioDevice for Uart {
    fn try_new(device_tree: &Fdt, compatibles: &[&str]) -> Option<Self> {
        let region = device_tree
            .find_compatible(compatibles)?
            .reg()
            .unwrap()
            .next()
            .unwrap();

        UART_ADDR
            .lock()
            .get_or_init(|| HostPhysicalAddress(region.starting_address as usize));

        Some(Uart {
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
