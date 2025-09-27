//! Milk-V Megrez
//!
//! Milk-V Megrez is a Mini-ITX device powered by the ESWIN EIC7700X.
//! [https://milkv.io/docs/megrez/overview](https://milkv.io/docs/megrez/overview)

/// Guest kernel image
#[unsafe(link_section = ".guest_kernel")]
pub static GUEST_KERNEL: [u8; include_bytes!("../../guest_image/megrez/vmlinux").len()] =
    *include_bytes!("../../guest_image/megrez/vmlinux");

/// Device tree blob for hart id 0 that is passed to guest
#[unsafe(link_section = ".guest_dtb")]
pub static GUEST_DTB_CORE0: [u8; include_bytes!("../../guest_image/megrez/cpu0.dtb").len()] =
    *include_bytes!("../../guest_image/megrez/cpu0.dtb");
/// Device tree blob for hart id 1 that is passed to guest
#[unsafe(link_section = ".guest_dtb")]
pub static GUEST_DTB_CORE1: [u8; include_bytes!("../../guest_image/megrez/cpu1.dtb").len()] =
    *include_bytes!("../../guest_image/megrez/cpu1.dtb");
/// Device tree blob for hart id 2 that is passed to guest
#[unsafe(link_section = ".guest_dtb")]
pub static GUEST_DTB_CORE2: [u8; include_bytes!("../../guest_image/megrez/cpu2.dtb").len()] =
    *include_bytes!("../../guest_image/megrez/cpu2.dtb");
/// Device tree blob for hart id 3 that is passed to guest
#[unsafe(link_section = ".guest_dtb")]
pub static GUEST_DTB_CORE3: [u8; include_bytes!("../../guest_image/megrez/cpu3.dtb").len()] =
    *include_bytes!("../../guest_image/megrez/cpu3.dtb");
