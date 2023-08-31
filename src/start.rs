use crate::memmap;
use crate::uart;
use riscv::register::{sstatus, stvec};

/// Start function
pub fn start(hart_id: u64, dtb_addr: u64) {
    unsafe {
        // clear sstatus.sie
        sstatus::clear_sie();

        // register panic_handler to stvec
        stvec::write(
            panic_handler as *const fn() as usize,
            stvec::TrapMode::Direct,
        );
    }

    let device_tree = unsafe {
        match fdt::Fdt::from_ptr(dtb_addr as *const u8) {
            Ok(fdt) => fdt,
            Err(e) => panic!("{}", e),
        }
    };

    let mmap = memmap::Memmap::new(device_tree);
    let uart = uart::Uart::new(mmap.uart_addr as u64);
}

/// Panic handler
fn panic_handler() {}
