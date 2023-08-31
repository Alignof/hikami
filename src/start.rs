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

        let device_tree = match fdt::Fdt::from_ptr(dtb_addr as *const u8) {
            Ok(fdt) => fdt,
            Err(e) => panic!("{}", e),
        };

        let uart_addr = device_tree
            .find_node("/soc/uart")
            .unwrap()
            .reg()
            .unwrap()
            .next()
            .unwrap()
            .starting_address;

        let uart = uart::Uart::new(uart_addr as u64);
    }
}

/// Panic handler
fn panic_handler() {}
