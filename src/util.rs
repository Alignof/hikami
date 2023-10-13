use crate::memmap::constant::UART_ADDR;
use core::fmt::{self, Write};

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::util::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    ($fmt:expr) => (print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (print!(concat!($fmt, "\n"), $($arg)*));
}

pub fn _print(args: fmt::Arguments) {
    let mut writer = UartWriter {};
    writer.write_fmt(args).unwrap();
}

struct UartWriter;

impl Write for UartWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let uart_addr = UART_ADDR as *mut u32;
        for c in s.bytes() {
            unsafe {
                while (uart_addr.read_volatile() as i32) < 0 {}
                uart_addr.write_volatile(c as u32);
            }
        }
        Ok(())
    }
}
