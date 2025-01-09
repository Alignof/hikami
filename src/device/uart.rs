//! UART: Universal Asynchronous Receiver-Transmitter

use super::{MmioDevice, PTE_FLAGS_FOR_DEVICE};
use crate::memmap::{GuestPhysicalAddress, HostPhysicalAddress, MemoryMap};

use core::cell::OnceCell;
use core::fmt::{self, Write};
use fdt::Fdt;
use rustsbi::{Physical, SbiRet};
use spin::Mutex;

mod register {
    //! Ref: [http://byterunner.com/16550.html](http://byterunner.com/16550.html)

    /// LSR register offset.
    pub const LSR_OFFSET: usize = 3;
}

/// Uart address for `UartWriter`.
static UART_ADDR: Mutex<OnceCell<HostPhysicalAddress>> = Mutex::new(OnceCell::new());

/// Struct for `Write` trait.
struct UartWriter;

impl Write for UartWriter {
    /// Write string to tty via UART.
    #[allow(clippy::cast_possible_wrap)]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let uart_addr = UART_ADDR.lock().get().unwrap().raw() as *mut u32;
        for c in s.bytes() {
            unsafe {
                while (uart_addr.read_volatile() as i32) < 0 {}
                uart_addr.write_volatile(u32::from(c));
            }
        }
        Ok(())
    }
}

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
    fn new(device_tree: &Fdt, node_path: &str) -> Self {
        let region = device_tree
            .find_node(node_path)
            .unwrap()
            .reg()
            .unwrap()
            .next()
            .unwrap();

        UART_ADDR
            .lock()
            .get_or_init(|| HostPhysicalAddress(region.starting_address as usize));

        Uart {
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

/// Ref: [https://docs.rs/rustsbi/0.4.0-alpha.1/rustsbi/trait.Console.html](https://docs.rs/rustsbi/0.4.0-alpha.1/rustsbi/trait.Console.html)
///
/// It doesn't seems to be used by linux.
/// TODO: Checking target address?
impl rustsbi::Console for Uart {
    /// Write bytes to the debug console from input memory.
    fn write(&self, bytes: Physical<&[u8]>) -> SbiRet {
        let uart_ptr = self.base_addr.raw() as *mut u32;
        let uart_lsr_ptr = self.lsr_addr().raw() as *mut u32;
        let byte_data = unsafe {
            core::slice::from_raw_parts(bytes.phys_addr_lo() as *const u8, bytes.num_bytes())
        };
        for c in byte_data {
            unsafe {
                while ((uart_lsr_ptr.read_volatile() >> 5) & 0x1) == 1 {}
                uart_ptr.write_volatile(u32::from(*c));
            }
        }
        SbiRet::success(0)
    }

    /// Read bytes from the debug console into an output memory.
    #[allow(clippy::cast_possible_truncation)]
    fn read(&self, bytes: Physical<&mut [u8]>) -> SbiRet {
        let uart_ptr = self.base_addr.raw() as *mut u32;
        let uart_lsr_ptr = self.lsr_addr().raw() as *mut u32;
        let buffer = unsafe {
            core::slice::from_raw_parts_mut(bytes.phys_addr_lo() as *mut u8, bytes.num_bytes())
        };

        let mut count = 0usize;
        unsafe {
            for c in buffer {
                if uart_lsr_ptr.read_volatile() & 0x1 == 1 {
                    *c = uart_ptr.read_volatile() as u8;
                    count += 1;
                } else {
                    break;
                }
            }
        }
        SbiRet::success(count)
    }

    /// Write a single byte to the debug console.
    fn write_byte(&self, byte: u8) -> SbiRet {
        let uart_ptr = self.base_addr.raw() as *mut u32;
        let uart_lsr_ptr = self.lsr_addr().raw() as *mut u32;
        unsafe {
            while ((uart_lsr_ptr.read_volatile() >> 5) & 0x1) == 1 {}
            uart_ptr.write_volatile(u32::from(byte));
        }
        SbiRet::success(0)
    }
}
