//! Symbols defined in the linker script (`link.x` and `linker_scripts/<board>.x`).

unsafe extern "C" {
    /// Top of the trap stack.
    pub static _stack_start: u8;
    /// Start of the heap.
    pub static mut _start_heap: u8;
    /// Size of the heap.
    pub static _hv_heap_size: u8;
    /// Top of the boot stack.
    pub static _top_b_stack: u8;
    /// Start of bss and sbss section.
    pub static _start_bss: u8;
    /// End of bss and sbss section.
    pub static _end_bss: u8;
    /// Start of the range occupied by the hypervisor.
    pub static _hypervisor_start: u8;
    /// End of the range occupied by the hypervisor.
    pub static _hypervisor_end: u8;
}
