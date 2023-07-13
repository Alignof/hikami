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
    ld sp, stacks_addr
    /* Now jump to the rust world; __start_rust.  */
    j       __start_rust

.bss
stacks:
    .skip 1024

.data
stacks_addr: .dword stacks
"#
);

#[no_mangle]
pub extern "C" fn __start_rust() -> ! {
    let uart = 0x1001_1000 as *mut u8;
    for c in b"Hello from Rust!".iter() {
        unsafe {
            *uart = *c as u8;
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
