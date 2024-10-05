//! Extension emulation

pub mod zicfiss;

use crate::h_extension::csrs::vstvec;
use crate::trap::hypervisor_supervisor::hstrap_exit;
use crate::HYPERVISOR_DATA;

use core::arch::asm;

/// Throw an VS-level exception.
/// * `exception_num`: Exception number. (store to vscause)
/// * `trap_value`: Trap value. (store to vstval)
pub fn pseudo_vs_exception(exception_num: usize, trap_value: usize) {
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

        context.set_sepc(vstvec::read().bits());

        drop(hypervisor_data);

        hstrap_exit();
    }
}
