//! QEMU
//!
//! ```no_run
//! $ qemu-system-riscv64 --version
//! QEMU emulator version 10.0.2
//! Copyright (c) 2003-2025 Fabrice Bellard and the QEMU Project developers
//! ```

/// Guest kernel image
#[unsafe(link_section = ".guest_kernel")]
pub static GUEST_KERNEL: [u8; include_bytes!("../../guest_image/qemu/emulation_eval").len()] =
    *include_bytes!("../../guest_image/qemu/emulation_eval");

/// Device tree blob for hart id 0 that is passed to guest
#[unsafe(link_section = ".guest_dtb")]
pub static GUEST_DTB_CORE0: [u8; include_bytes!("../../guest_image/qemu/cpu0.dtb").len()] =
    *include_bytes!("../../guest_image/qemu/cpu0.dtb");
/// Device tree blob for hart id 1 that is passed to guest
#[unsafe(link_section = ".guest_dtb")]
pub static GUEST_DTB_CORE1: [u8; include_bytes!("../../guest_image/qemu/cpu1.dtb").len()] =
    *include_bytes!("../../guest_image/qemu/cpu1.dtb");
/// Device tree blob for hart id 2 that is passed to guest
#[unsafe(link_section = ".guest_dtb")]
pub static GUEST_DTB_CORE2: [u8; include_bytes!("../../guest_image/qemu/cpu2.dtb").len()] =
    *include_bytes!("../../guest_image/qemu/cpu2.dtb");
/// Device tree blob for hart id 3 that is passed to guest
#[unsafe(link_section = ".guest_dtb")]
pub static GUEST_DTB_CORE3: [u8; include_bytes!("../../guest_image/qemu/cpu3.dtb").len()] =
    *include_bytes!("../../guest_image/qemu/cpu3.dtb");
