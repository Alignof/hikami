//! UART: Universal Asynchronous Receiver-Transmitter

use super::MmioDevice;
use crate::memmap::HostPhysicalAddress;

use alloc::vec::Vec;
use core::cell::OnceCell;
use fdt::{Fdt, standard_nodes::MemoryRegion};
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
    /// Memory maps for memory mapped register.
    register_map_regions: Vec<MemoryRegion>,
}

impl Uart {
    /// Return address of LSR register.
    #[must_use]
    pub fn lsr_addr(&self) -> HostPhysicalAddress {
        HostPhysicalAddress(self.register_map_regions[0].starting_address as usize)
            + register::LSR_OFFSET
    }
}

impl MmioDevice for Uart {
    fn try_new(
        root_page_table_addr: HostPhysicalAddress,
        device_tree: &Fdt,
        compatibles: &[&str],
    ) -> Option<Self> {
        let uart_node = device_tree.find_compatible(compatibles).unwrap_or_else(|| {
            device_tree
                .find_node("/soc/serial")
                .expect("uart is not found in fdt")
        });
        let register_map_regions: Vec<MemoryRegion> = uart_node.reg().unwrap().collect();

        Self::create_page_table(root_page_table_addr, &register_map_regions, uart_node.name);

        UART_ADDR
            .lock()
            .get_or_init(|| HostPhysicalAddress(register_map_regions[0].starting_address as usize));

        Some(Uart {
            register_map_regions,
        })
    }
}
