use super::Device;
use crate::memmap::page_table::PteFlag;
use crate::memmap::{constant, MemoryMap};
use fdt::Fdt;
use rustsbi::{Physical, SbiRet};

mod register {
    //! Ref: [http://byterunner.com/16550.html](http://byterunner.com/16550.html)

    /// LSR register offset.
    pub const LSR_OFFSET: usize = 3;
}

const DEVICE_FLAGS: [PteFlag; 5] = [
    PteFlag::Dirty,
    PteFlag::Accessed,
    PteFlag::Write,
    PteFlag::Read,
    PteFlag::Valid,
];

/// UART: Universal asynchronous receiver-transmitter
#[derive(Debug)]
pub struct Uart {
    base_addr: usize,
    size: usize,
}

impl Uart {
    pub fn lsr_addr(&self) -> usize {
        self.base_addr + register::LSR_OFFSET
    }
}

impl Device for Uart {
    fn new(device_tree: &Fdt, node_path: &str) -> Self {
        let region = device_tree
            .find_node(node_path)
            .unwrap()
            .reg()
            .unwrap()
            .next()
            .unwrap();

        Uart {
            base_addr: region.starting_address as usize,
            size: region.size.unwrap(),
        }
    }

    fn size(&self) -> usize {
        self.size
    }

    fn paddr(&self) -> usize {
        self.base_addr
    }

    fn vaddr(&self) -> usize {
        self.base_addr + constant::PA2VA_DEVICE_OFFSET
    }

    fn memmap(&self) -> MemoryMap {
        MemoryMap::new(
            self.vaddr()..self.vaddr() + self.size(),
            self.paddr()..self.paddr() + self.size(),
            &DEVICE_FLAGS,
        )
    }

    fn identity_memmap(&self) -> MemoryMap {
        MemoryMap::new(
            self.paddr()..self.paddr() + self.size(),
            self.paddr()..self.paddr() + self.size(),
            &DEVICE_FLAGS,
        )
    }
}

/// Ref: [https://docs.rs/rustsbi/0.4.0-alpha.1/rustsbi/trait.Console.html](https://docs.rs/rustsbi/0.4.0-alpha.1/rustsbi/trait.Console.html)
///
/// TODO: Checking target address range?
impl rustsbi::Console for Uart {
    /// Write bytes to the debug console from input memory.
    fn write(&self, bytes: Physical<&[u8]>) -> SbiRet {
        let uart_addr = self.base_addr as *mut u32;
        let uart_lsr_addr = self.lsr_addr() as *mut u32;
        let byte_data = unsafe {
            core::slice::from_raw_parts(bytes.phys_addr_lo() as *const u8, bytes.num_bytes())
        };
        for c in byte_data {
            unsafe {
                while (uart_lsr_addr.read_volatile() >> 5 & 0x1) == 1 {}
                uart_addr.write_volatile(u32::from(*c));
            }
        }
        SbiRet::success(0)
    }

    /// Read bytes from the debug console into an output memory.
    fn read(&self, bytes: Physical<&mut [u8]>) -> SbiRet {
        let uart_addr = self.base_addr as *mut u32;
        let uart_lsr_addr = self.lsr_addr() as *mut u32;
        let buffer = unsafe {
            core::slice::from_raw_parts_mut(bytes.phys_addr_lo() as *mut u8, bytes.num_bytes())
        };

        let mut count = 0usize;
        unsafe {
            for c in buffer {
                if uart_lsr_addr.read_volatile() & 0x1 == 1 {
                    *c = uart_addr.read_volatile() as u8;
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
        let uart_addr = self.base_addr as *mut u32;
        let uart_lsr_addr = self.lsr_addr() as *mut u32;
        unsafe {
            while (uart_lsr_addr.read_volatile() >> 5 & 0x1) == 1 {}
            uart_addr.write_volatile(u32::from(byte));
        }
        SbiRet::success(0)
    }
}
