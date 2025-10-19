//! Trap VS-mode interrupt.

use super::hstrap_exit;
use hikami_core::HYPERVISOR_DATA;
use hikami_core::device::plic::ContextId;
use hikami_core::guest::context::ContextData;
use hikami_core::h_extension::csrs::{VsInterruptKind, hvip};

use riscv::register::scause::Interrupt;
use riscv::register::sie;

/// Trap handler for Interrupt
#[allow(clippy::module_name_repetitions)]
pub fn trap_interrupt(interrupt_cause: Interrupt) -> ! {
    match interrupt_cause {
        Interrupt::SupervisorSoft => unsafe {
            hvip::set(VsInterruptKind::Software);
            sie::clear_ssoft();
        },
        Interrupt::SupervisorTimer => unsafe {
            // hikami_core::debugln!("[DEBUG] timer interrupt occured!!!!");
            let hypervisor_data = HYPERVISOR_DATA.lock();
            let stack_top = hypervisor_data.get().unwrap().guest().stack_top();

            hvip::set(VsInterruptKind::Timer);
            sie::clear_stimer();
            /*
            unsafe {
                let current_instret: u64;
                let interrupt_instret: u64;

                core::arch::asm!("
                    rdinstret {current_instret}

                    // set to stack top
                    mv t5, {stack_top}
                    addi t5, t5, -{HS_CONTEXT_SIZE}
                    ld {interrupt_instret}, 34*8(t5)
                    ",
                    HS_CONTEXT_SIZE = const size_of::<ContextData>(),
                    stack_top = in(reg) stack_top.raw(),
                    current_instret = out(reg) current_instret,
                    interrupt_instret = out(reg) interrupt_instret,
                );
                // hikami_core::debugln!("interrupt_instret: {}", interrupt_instret);
                // hikami_core::debugln!("current_instret: {}", current_instret);
                // hikami_core::debugln!(
                //     "current_instret - interrupt_instret: {}",
                //     current_instret - interrupt_instret
                // );
                hikami_core::debugln!(
                    "[Timer interrupt] 37 + current_instret - 2 - interrupt_instret + 40: {}",
                    37 // instructions before get interrupt_instret value
                        + current_instret // current instret value
                        - 2 // rdinstret t0, sd t0, 34*8(sp)
                        - interrupt_instret // instret value when entered interrupt handler
                        + 40 // remain instructions to exit
                );
            }
            */
        },
        Interrupt::SupervisorExternal => unsafe {
            // hikami_core::debugln!("[DEBUG] external interrupt occured!!!!");
            let mut hypervisor_data = HYPERVISOR_DATA.lock();
            let hart_id = hypervisor_data.get().unwrap().guest().hart_id();
            let context_id = ContextId::new(hart_id, true);

            // read plic claim/update register and reflect to plic.claim_complete.
            hypervisor_data
                .get_mut()
                .unwrap()
                .devices()
                .plic
                .update_claim_complete(&context_id);

            hvip::set(VsInterruptKind::External);
            sie::clear_sext();
            /*
            unsafe {
                let current_instret: u64;
                let interrupt_instret: u64;

                core::arch::asm!("
                    rdinstret {current_instret}

                    // set to stack top
                    mv t5, {stack_top}
                    addi t5, t5, -{HS_CONTEXT_SIZE}
                    ld {interrupt_instret}, 34*8(t5)
                    ",
                    HS_CONTEXT_SIZE = const size_of::<ContextData>(),
                    stack_top = in(reg) stack_top.raw(),
                    current_instret = out(reg) current_instret,
                    interrupt_instret = out(reg) interrupt_instret,
                );
                hikami_core::debugln!(
                    "[External interrupt] 37 + current_instret - 2 - interrupt_instret + 40: {}",
                    37 // instructions before get interrupt_instret value
                        + current_instret // current instret value
                        - 2 // rdinstret t0, sd t0, 34*8(sp)
                        - interrupt_instret // instret value when entered interrupt handler
                        + 40 // remain instructions to exit
                );
            }
            */
        },
        Interrupt::Unknown => panic!("unknown interrupt type"),
    }

    unsafe {
        hstrap_exit();
    }
}
