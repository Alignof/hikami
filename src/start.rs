use riscv::register::{sstatus, stvec};

/// Start function
pub fn start(hart_id: u64, dtb_addr: u64) {
    unsafe {
        sstatus::clear_sie();
        stvec::write(
            panic_handler as *const fn() as usize,
            stvec::TrapMode::Direct,
        );
    }
}

/// Panic handler
fn panic_handler() {}
