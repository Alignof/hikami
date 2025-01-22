//! Handle page fault exceptions.
//!
//! - Load guest page fault
//! - Store AMO guest page fault

use super::{hs_forward_exception, update_sepc_by_htinst_value};
use crate::h_extension::csrs::{htinst, htval};
use crate::memmap::page_table::g_stage_trans_addr;
use crate::memmap::{GuestPhysicalAddress, HostPhysicalAddress};
use crate::HYPERVISOR_DATA;

use raki::Instruction;

/// Fetch fault instruction
fn fetch_fault_inst(fault_addr: HostPhysicalAddress) -> usize {
    let inst_value = unsafe { (fault_addr.raw() as *const u32).read_volatile() };
    if inst_value & 0b11 == 0b11 {
        inst_value as usize
    } else {
        (inst_value & 0xffff) as usize
    }
}

/// Trap `Load guest page fault` exception.
pub fn load_guest_page_fault() {
    let fault_addr = GuestPhysicalAddress(htval::read().bits() << 2);

    let fault_inst_value = htinst::read().bits();
    // htinst bit 1 replaced with a 0.
    // thus it needed to flip bit 1.
    // ref: vol. II p.161
    let fault_inst = if fault_inst_value == 0 {
        let fault_addr = g_stage_trans_addr(fault_addr).expect("not a identity map");
        let fault_inst_value = fetch_fault_inst(fault_addr);
        assert_ne!(fault_inst_value, 0);
        Instruction::try_from(fault_inst_value).expect("decoding load fault instruction failed")
    } else {
        Instruction::try_from(fault_inst_value | 0b10)
            .expect("decoding load fault instruction failed")
    };

    let mut hypervisor_data = unsafe { HYPERVISOR_DATA.lock() };
    if let Ok(value) = hypervisor_data
        .get_mut()
        .unwrap()
        .devices()
        .plic
        .emulate_loading(HostPhysicalAddress(fault_addr.raw()))
    {
        let mut context = hypervisor_data.get().unwrap().guest().context;
        context.set_xreg(fault_inst.rd.expect("rd is not found"), u64::from(value));
        update_sepc_by_htinst_value(fault_inst_value, &mut context);
        return;
    }

    if let Some(pci) = &mut hypervisor_data.get_mut().unwrap().devices().pci {
        if let Some(sata) = &pci.pci_devices.sata {
            if let Ok(value) = sata.emulate_loading(HostPhysicalAddress(fault_addr.raw())) {
                let mut context = hypervisor_data.get().unwrap().guest().context;
                context.set_xreg(fault_inst.rd.expect("rd is not found"), u64::from(value));
                update_sepc_by_htinst_value(fault_inst_value, &mut context);
                return;
            }
        }
    }

    drop(hypervisor_data);
    hs_forward_exception();
}

/// Trap `Store guest page fault` exception.
#[allow(clippy::cast_possible_truncation)]
pub fn store_guest_page_fault() {
    let fault_addr = GuestPhysicalAddress(htval::read().bits() << 2);

    let fault_inst_value = htinst::read().bits();
    // htinst bit 1 replaced with a 0.
    // thus it needed to flip bit 1.
    // ref: vol. II p.161
    let fault_inst = if fault_inst_value == 0 {
        let fault_addr = g_stage_trans_addr(fault_addr).expect("not a identity map");
        let fault_inst_value = fetch_fault_inst(fault_addr);
        assert_ne!(fault_inst_value, 0);
        Instruction::try_from(fault_inst_value).expect("decoding load fault instruction failed")
    } else {
        Instruction::try_from(fault_inst_value | 0b10)
            .expect("decoding load fault instruction failed")
    };

    let mut hypervisor_data = unsafe { HYPERVISOR_DATA.lock() };
    let mut context = hypervisor_data.get().unwrap().guest().context;
    //let store_value = context.xreg(fault_inst.rs2.expect("rs2 is not found"));
    let store_value = context.xreg(match fault_inst.rs2 {
        Some(x) => x,
        None => panic!("rs2 is not found: {fault_inst:#?} (inst_value: {fault_inst_value})"),
    });

    if let Ok(()) = hypervisor_data
        .get_mut()
        .unwrap()
        .devices()
        .plic
        .emulate_storing(HostPhysicalAddress(fault_addr.raw()), store_value as u32)
    {
        update_sepc_by_htinst_value(fault_inst_value, &mut context);
        return;
    }

    if let Some(pci) = &mut hypervisor_data.get_mut().unwrap().devices().pci {
        if let Some(sata) = &mut pci.pci_devices.sata {
            if let Ok(()) =
                sata.emulate_storing(HostPhysicalAddress(fault_addr.raw()), store_value as u32)
            {
                update_sepc_by_htinst_value(fault_inst_value, &mut context);
                return;
            }
        }
    }

    drop(hypervisor_data);
    hs_forward_exception();
}
