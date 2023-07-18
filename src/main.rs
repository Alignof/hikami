#![no_main]
#![no_std]

use core::arch::global_asm;

global_asm!(
    r#"
.option norvc
.section .reset.boot, "ax",@progbits
.global _start
.global abort

_start:
    /* Set up stack pointer. */
    lla      sp, stacks_end
    /* Now jump to the rust world; __start_rust.  */
    j       __start_rust

.bss
stacks:
    .skip 1024
stacks_end:
"#
);

#[no_mangle]
pub extern "C" fn __start_rust() -> ! {
    let uart = 0x1001_0000 as *mut u32;

    for c in b"Hello from Rust!\n".iter() {
        unsafe {
            while (uart.read_volatile() as i32) < 0 {}
            uart.write_volatile(*c as u32);
        }
    }

    loop {}
}

use core::panic::PanicInfo;
#[panic_handler]
#[no_mangle]
pub fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn abort() -> ! {
    loop {}
}
