//! Handle page fault exceptions.
//!
//! - Load guest page fault
//! - Store AMO guest page fault

use super::{hs_forward_exception, hstrap_exit, update_sepc_by_htinst_value};
use crate::device::DeviceEmulateError;
use crate::h_extension::csrs::{htinst, htval};
use crate::memmap::HostPhysicalAddress;
use crate::HYPERVISOR_DATA;

use raki::Instruction;

/// Trap `Load guest page fault` exception.
pub fn load_guest_page_fault() {
    let fault_addr = HostPhysicalAddress(htval::read().bits() << 2);
    let fault_inst_value = htinst::read().bits();
    // htinst bit 1 replaced with a 0.
    // thus it needed to flip bit 1.
    // ref: vol. II p.161
    let fault_inst = Instruction::try_from(fault_inst_value | 0b10)
        .expect("decoding load fault instruction failed");

    let mut hypervisor_data = unsafe { HYPERVISOR_DATA.lock() };
    if let Ok(value) = hypervisor_data
        .get_mut()
        .unwrap()
        .devices()
        .plic
        .emulate_read(fault_addr)
    {
        let mut context = hypervisor_data.get().unwrap().guest().context;
        context.set_xreg(fault_inst.rd.expect("rd is not found"), u64::from(value));
        update_sepc_by_htinst_value(fault_inst_value, &mut context);
        return;
    }

    if let Some(sata) = &mut hypervisor_data
        .get_mut()
        .unwrap()
        .devices()
        .pci
        .pci_devices
        .sata
    {
        if let Ok(value) = sata.emulate_read(fault_addr) {
            let mut context = hypervisor_data.get().unwrap().guest().context;
            context.set_xreg(fault_inst.rd.expect("rd is not found"), u64::from(value));
            update_sepc_by_htinst_value(fault_inst_value, &mut context);
            return;
        }
    }

    drop(hypervisor_data);
    hs_forward_exception();
}

/// Trap `Store guest page fault` exception.
pub fn store_guest_page_fault() {
    let fault_addr = HostPhysicalAddress(htval::read().bits() << 2);
    let fault_inst_value = htinst::read().bits();
    // htinst bit 1 replaced with a 0.
    // thus it needed to flip bit 1.
    // ref: vol. II p.161
    let fault_inst = Instruction::try_from(fault_inst_value | 0b10)
        .expect("decoding load fault instruction failed");

    let mut hypervisor_data = unsafe { HYPERVISOR_DATA.lock() };
    let mut context = hypervisor_data.get().unwrap().guest().context;
    //let store_value = context.xreg(fault_inst.rs2.expect("rs2 is not found"));
    let store_value = context.xreg(match fault_inst.rs2 {
        Some(x) => x,
        None => panic!("rs2 is not found: {fault_inst:#?}"),
    });

    if let Ok(()) = hypervisor_data
        .get_mut()
        .unwrap()
        .devices()
        .plic
        .emulate_write(fault_addr, store_value as u32)
    {
        update_sepc_by_htinst_value(fault_inst_value, &mut context);
        return;
    }

    if let Some(sata) = &mut hypervisor_data
        .get_mut()
        .unwrap()
        .devices()
        .pci
        .pci_devices
        .sata
    {
        if let Ok(()) = sata.emulate_write(fault_addr, store_value as u32) {
            update_sepc_by_htinst_value(fault_inst_value, &mut context);
            return;
        }
    }

    drop(hypervisor_data);
    hs_forward_exception();
}
