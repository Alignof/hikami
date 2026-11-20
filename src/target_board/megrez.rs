//! Milk-V Megrez
//!
//! Milk-V Megrez is a Mini-ITX device powered by the ESWIN EIC7700X.
//! [https://milkv.io/docs/megrez/overview](https://milkv.io/docs/megrez/overview)

use hikami_core::board::Board;

/// Guest kernel image
#[unsafe(link_section = ".guest_kernel")]
static GUEST_KERNEL: [u8; include_bytes!("../../guest_image/megrez/vmlinux").len()] =
    *include_bytes!("../../guest_image/megrez/vmlinux");

/// Device tree blob for hart id 0 that is passed to guest
#[unsafe(link_section = ".guest_dtb")]
static GUEST_DTB_CORE0: [u8; include_bytes!("../../guest_image/megrez/cpu0.dtb").len()] =
    *include_bytes!("../../guest_image/megrez/cpu0.dtb");
/// Device tree blob for hart id 1 that is passed to guest
#[unsafe(link_section = ".guest_dtb")]
static GUEST_DTB_CORE1: [u8; include_bytes!("../../guest_image/megrez/cpu1.dtb").len()] =
    *include_bytes!("../../guest_image/megrez/cpu1.dtb");
/// Device tree blob for hart id 2 that is passed to guest
#[unsafe(link_section = ".guest_dtb")]
static GUEST_DTB_CORE2: [u8; include_bytes!("../../guest_image/megrez/cpu2.dtb").len()] =
    *include_bytes!("../../guest_image/megrez/cpu2.dtb");
/// Device tree blob for hart id 3 that is passed to guest
#[unsafe(link_section = ".guest_dtb")]
static GUEST_DTB_CORE3: [u8; include_bytes!("../../guest_image/megrez/cpu3.dtb").len()] =
    *include_bytes!("../../guest_image/megrez/cpu3.dtb");

/// Board configuration of Milk-V Megrez.
pub static BOARD: Board = Board {
    guest_kernel: &GUEST_KERNEL,
    guest_dtbs: &[
        &GUEST_DTB_CORE0,
        &GUEST_DTB_CORE1,
        &GUEST_DTB_CORE2,
        &GUEST_DTB_CORE3,
    ],
    guest_initrd: &crate::GUEST_INITRD,
};
