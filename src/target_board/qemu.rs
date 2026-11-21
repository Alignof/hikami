//! QEMU
//!
//! ```no_run
//! $ qemu-system-riscv64 --version
//! QEMU emulator version 10.0.2
//! Copyright (c) 2003-2025 Fabrice Bellard and the QEMU Project developers
//! ```

use hikami_core::board::Board;
use hikami_core::memmap::GuestPhysicalAddress;

/// Guest kernel image
#[unsafe(link_section = ".guest_kernel")]
static GUEST_KERNEL: [u8; include_bytes!("../../guest_image/qemu/vmlinux").len()] =
    *include_bytes!("../../guest_image/qemu/vmlinux");

/// Device tree blob for hart id 0 that is passed to guest
#[unsafe(link_section = ".guest_dtb")]
static GUEST_DTB_CORE0: [u8; include_bytes!("../../guest_image/qemu/cpu0.dtb").len()] =
    *include_bytes!("../../guest_image/qemu/cpu0.dtb");
/// Device tree blob for hart id 1 that is passed to guest
#[unsafe(link_section = ".guest_dtb")]
static GUEST_DTB_CORE1: [u8; include_bytes!("../../guest_image/qemu/cpu1.dtb").len()] =
    *include_bytes!("../../guest_image/qemu/cpu1.dtb");
/// Device tree blob for hart id 2 that is passed to guest
#[unsafe(link_section = ".guest_dtb")]
static GUEST_DTB_CORE2: [u8; include_bytes!("../../guest_image/qemu/cpu2.dtb").len()] =
    *include_bytes!("../../guest_image/qemu/cpu2.dtb");
/// Device tree blob for hart id 3 that is passed to guest
#[unsafe(link_section = ".guest_dtb")]
static GUEST_DTB_CORE3: [u8; include_bytes!("../../guest_image/qemu/cpu3.dtb").len()] =
    *include_bytes!("../../guest_image/qemu/cpu3.dtb");

/// Board configuration of QEMU virt.
pub static BOARD: Board = Board {
    guest_kernel: &GUEST_KERNEL,
    guest_kernel_base: GuestPhysicalAddress(0x1_0000_0000),
    guest_dtbs: &[
        &GUEST_DTB_CORE0,
        &GUEST_DTB_CORE1,
        &GUEST_DTB_CORE2,
        &GUEST_DTB_CORE3,
    ],
    guest_dtb_base: GuestPhysicalAddress(0x8000_0000),
    guest_initrd: &crate::GUEST_INITRD,
};
