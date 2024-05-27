use crate::csrs::{hvip, vsatp};

#[inline(never)]
pub extern "C" fn init_hypervisor(hart_id: usize, _dtb_addr: usize) {
    // hart_id must be zero.
    assert_eq!(hart_id, 0);

    // clear all hypervisor interrupts.
    hvip::write(0);

    // disable address translation.
    vsatp::write(0);
}
