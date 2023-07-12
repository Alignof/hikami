#![no_main]
#![no_std]

use core::arch::global_asm;

#[link_section = ".boot"]
global_asm!(
    r#"
.global _start
_start:
    /* Set up stack pointer. */
    lui     sp, %hi(stack_end)
    ori     sp, sp, %lo(stack_end)

    /* Now jump to the rust world; __start_rust.  */
    j       __start_rust

.bss

stack_start:
    .skip 1024
stack_end:
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
