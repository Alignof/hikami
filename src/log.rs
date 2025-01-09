//! Print macros for logging

use core::fmt::{self, Write};
use sbi_rt::Physical;

/// Writer for print macro.
struct Writer;
impl core::fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        sbi_rt::console_write(Physical::new(
            s.len(),
            s.as_ptr() as usize & 0xffff_ffff,
            (s.as_ptr() as usize >> 32) & 0xffff_ffff,
        ));
        Ok(())
    }
}

/// Print function calling from print macro
pub fn print_for_macro(args: fmt::Arguments) {
    let mut writer = Writer;
    writer.write_fmt(args).unwrap();
}

/// Print to standard output.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::log::print_for_macro(format_args!($($arg)*)));
}

/// Print with linebreak to standard output.
#[macro_export]
macro_rules! println {
    ($fmt:expr) => ($crate::print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::print!(concat!($fmt, "\n"), $($arg)*));
}
