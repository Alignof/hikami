use crate::memmap::constant::device::UART_ADDR;
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

/// Print function calling from print macro
pub fn _print(args: fmt::Arguments) {
    let mut writer = UartWriter {};
    writer.write_fmt(args).unwrap();
}

struct UartWriter;

impl Write for UartWriter {
    /// Write string to tty via UART.
    #[allow(clippy::cast_possible_wrap)]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let uart_addr = UART_ADDR.raw() as *mut u32;
        for c in s.bytes() {
            unsafe {
                while (uart_addr.read_volatile() as i32) < 0 {}
                uart_addr.write_volatile(u32::from(c));
            }
        }
        Ok(())
    }
}
