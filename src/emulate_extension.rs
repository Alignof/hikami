//! Extension emulation

pub mod zicfiss;

use crate::h_extension::csrs::vstvec;
use crate::trap::hstrap_exit;
use crate::HYPERVISOR_DATA;

use core::arch::asm;
use riscv::register::sstatus;

/// Initialize singletons for extension emulation.
/// TODO: Remove it when `OnceCell` is replaced to `LazyCell`.
pub fn initialize() {
    use zicfiss::{Zicfiss, ZICFISS_DATA};
    unsafe { ZICFISS_DATA.lock() }.get_or_init(Zicfiss::new);
}

/// Throw an VS-level exception.
/// * `exception_num`: Exception number. (stored to vscause)
/// * `trap_value`: Trap value. (stored to vstval)
pub fn pseudo_vs_exception(exception_num: usize, trap_value: usize) -> ! {
    unsafe {
        let hypervisor_data = HYPERVISOR_DATA.lock();
        let mut context = hypervisor_data.get().unwrap().guest().context;
        asm!(
            "csrw vsepc, {sepc}",
            "csrw vscause, {cause}",
            "csrw vstval, {tval}",
            sepc = in(reg) context.sepc(),
            cause = in(reg) exception_num,
            tval = in(reg) trap_value,
        );

        let spp = sstatus::read().spp();
        let vsstatus: usize;
        asm!("csrr {status}, vsstatus", status = out(reg) vsstatus);
        let sie = (vsstatus >> 1) & 0x1;
        asm!(
            "csrw vsstatus, {status}",
            status = in(reg) (vsstatus & !(1 << 8)) | ((spp as usize) << 8)
        );
        // disable interrupt
        asm!(
            "csrs vsstatus, {status}",
            "csrci vsstatus, 0b10",
            status = in(reg) sie << 5,
        );
        context.set_sstatus(context.sstatus() | (1 << 8));

        context.set_sepc(vstvec::read().bits());

        drop(hypervisor_data);

        hstrap_exit();
    }
}
