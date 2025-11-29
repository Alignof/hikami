//! Trap VS-mode exception.

mod instruction_handler;
mod page_fault_handler;
mod sbi_handler;

use super::hstrap_exit;
use hikami_core::HYPERVISOR_DATA;
use hikami_core::guest;
use hikami_core::h_extension::{
    HvException,
    csrs::{htval, vstvec},
};

use core::arch::asm;
use riscv::register::{
    scause::{self, Exception},
    stval,
};
use sbi_handler::sbi_call;
use sbi_handler::{
    sbi_base_handler, sbi_fwft_handler, sbi_pmu_handler, sbi_rfnc_handler, sbi_time_handler,
};

/// Delegate exception to supervisor mode from VS-mode.
#[unsafe(no_mangle)]
#[allow(clippy::inline_always, clippy::module_name_repetitions)]
pub extern "C" fn hs_forward_exception() {
    unsafe {
        let mut context = HYPERVISOR_DATA.lock().get().unwrap().guest().context;
        asm!(
            "csrw vsepc, {sepc}",
            "csrw vscause, {scause}",
            "csrw vstval, {stval}",
            sepc = in(reg) context.sepc(),
            scause = in(reg) match scause::read().bits() {
                20 => 1, // Instruction access fault
                21 => 5, // Load access fault
                23 => 7, // Store/AMO access fault
                unhadled_cause => unimplemented!("cause: {unhadled_cause}"),
            },
            stval = in(reg) stval::read(),
        );

        context.set_sepc(vstvec::read().bits());
    }
}

/// Handler for Ecall from VS-mode exception
#[allow(clippy::cast_possible_truncation)]
fn sbi_vs_mode_handler(context: &mut guest::context::Context) {
    /// Extension ID of FWFT(Firmware Features) Extension.
    const EID_FWFT: usize = 0x4657_4654;

    let ext_id: usize = context.xreg(17) as usize;
    let func_id: usize = context.xreg(16) as usize;
    let arguments: &[u64; 5] = &[
        context.xreg(10),
        context.xreg(11),
        context.xreg(12),
        context.xreg(13),
        context.xreg(14),
    ];

    let sbiret = match ext_id {
        sbi_spec::base::EID_BASE => sbi_base_handler(func_id),
        sbi_spec::pmu::EID_PMU => sbi_pmu_handler(func_id, arguments),
        sbi_spec::rfnc::EID_RFNC => sbi_rfnc_handler(func_id, arguments),
        sbi_spec::time::EID_TIME => sbi_time_handler(func_id, arguments),
        EID_FWFT => sbi_fwft_handler(func_id, arguments),
        _ => sbi_call(ext_id, func_id, arguments),
    };

    context.set_xreg(10, sbiret.error as u64);
    context.set_xreg(11, sbiret.value as u64);
}

/// Update sepc by inst size (2 byte or 4 byte)
fn update_sepc_by_inst_type(is_compressed: bool, context: &mut guest::context::Context) {
    if is_compressed {
        // compressed instruction
        context.set_sepc(context.sepc() + 2);
    } else {
        // normal size instruction
        context.set_sepc(context.sepc() + 4);
    }
}

/// Trap handler for exception
#[allow(clippy::cast_possible_truncation, clippy::module_name_repetitions)]
pub fn trap_exception(exception_cause: Exception) -> ! {
    #[allow(unused_variables)]
    if cfg!(feature = "debug_log") && scause::read().bits() != 0xa {
        use hikami_core::memmap::page_table::g_stage_trans_addr;
        use hikami_core::memmap::{GuestPhysicalAddress, HostPhysicalAddress};
        let htval = htval::read().bits();

        // if !(0xc00_0000..0x1000_0000).contains(&(htval << 2)) {
        //     let scause = scause::read().bits();
        //     let stval = stval::read();
        //     let sepc = riscv::register::sepc::read();
        //     let htval_hpa = g_stage_trans_addr(GuestPhysicalAddress(htval << 2))
        //         .ok()
        //         .map(HostPhysicalAddress::raw);
        //     let htinst = hikami_core::h_extension::csrs::htinst::read().bits();

        //     hikami_core::debugln!("!!! EXCEPTION CAUGHT !!!");
        //     hikami_core::debugln!("sepc:   {:#x}", sepc);
        //     hikami_core::debugln!("scause: {:#x}", scause);
        //     hikami_core::debugln!("stval:  {:#x}", stval);
        //     hikami_core::debugln!("htval << 2:  {:#x}", htval << 2);
        //     hikami_core::debugln!("htval(hpa):  {:#x?}", htval_hpa);
        //     hikami_core::debugln!("htinst: {:#x}", htinst);
        // }
    }

    match exception_cause {
        Exception::IllegalInstruction => {
            #[allow(named_asm_labels)]
            unsafe {
                asm!(
                    ".global __measure_end",
                    "__before_illegal_instruction:",
                    options(nostack, preserves_flags, nomem)
                );
            }
            instruction_handler::illegal_instruction();
            #[allow(named_asm_labels)]
            unsafe {
                asm!(
                    ".global __measure_end",
                    "__after_illegal_instruction:",
                    options(nostack, preserves_flags, nomem)
                );
            }
            // use hikami_core::guest::context::ContextData;
            // use raki::Instruction;
            // use riscv::register::sepc;

            // let mut current_instret: u64 = unsafe {
            //     let mut current_instret: u64;
            //     core::arch::asm!("
            //         rdinstret {current_instret}
            //         ",
            //         current_instret = out(reg) current_instret,
            //     );

            //     current_instret
            // };

            // let hypervisor_data = unsafe { HYPERVISOR_DATA.lock() };
            // let stack_top = hypervisor_data.get().unwrap().guest().stack_top();
            // unsafe {
            //     let interrupt_instret: u64;

            //     core::arch::asm!("
            //         // set to stack top
            //         mv t5, {stack_top}
            //         addi t5, t5, -{HS_CONTEXT_SIZE}
            //         ld {interrupt_instret}, 34*8(t5)
            //         ",
            //         HS_CONTEXT_SIZE = const size_of::<ContextData>(),
            //         stack_top = in(reg) stack_top.raw(),
            //         interrupt_instret = out(reg) interrupt_instret,
            //     );
            //     let fault_inst_value = stval::read();
            //     let fault_inst = Instruction::try_from(fault_inst_value).unwrap_or_else(|_| {
            //         use hikami_core::memmap::GuestVirtualAddress;
            //         let gva = GuestVirtualAddress(sepc::read());
            //         let gpa = hikami_core::memmap::page_table::vs_stage_trans_addr(gva).unwrap();
            //         let hpa = hikami_core::memmap::page_table::g_stage_trans_addr(gpa).unwrap();

            //         panic!(
            //             "decoding load fault instruction failed: fault inst value: {fault_inst_value:#x} at {:#x}(GPA: {:#x}, HPA: {:#x})",
            //             sepc::read(), gpa.raw(), hpa.raw()
            //         );
            //     });
            //     if let raki::OpcodeKind::Zbs(_) = fault_inst.opc {
            //         hikami_core::println!("current_instret {}", current_instret);
            //         hikami_core::println!("interrupt_instret {}", interrupt_instret);
            //         hikami_core::println!(
            //             "[Emulation] 37 + current_instret - 2 - interrupt_instret + 40: {}",
            //             37 // instructions before get interrupt_instret value
            //             + current_instret // current instret value
            //             - 2 // rdinstret t0, sd t0, 34*8(sp)
            //             - interrupt_instret // instret value when entered interrupt handler
            //             + 40 // remain instructions to exit
            //         );
            //     }
            // }
        }
        Exception::SupervisorEnvCall => panic!("SupervisorEnvCall should be handled by M-mode"),
        // Enum not found in `riscv` crate.
        Exception::Unknown => match HvException::from(scause::read().code()) {
            HvException::EcallFromVsMode => {
                let mut context = unsafe { HYPERVISOR_DATA.lock().get().unwrap().guest().context };
                sbi_vs_mode_handler(&mut context);
                context.set_sepc(context.sepc() + 4);
            }
            HvException::InstructionGuestPageFault => {
                panic!(
                    "Instruction guest-page fault\nfault gpa: {:#x}\nfault hpa: {:#x}",
                    stval::read(),
                    htval::read().bits() << 2,
                );
            }
            HvException::LoadGuestPageFault => page_fault_handler::load_guest_page_fault(),
            HvException::StoreAmoGuestPageFault => page_fault_handler::store_guest_page_fault(),
            HvException::VirtualInstruction => instruction_handler::virtual_instruction(),
        },
        _ => hs_forward_exception(),
    }

    unsafe {
        hstrap_exit();
    }
}
