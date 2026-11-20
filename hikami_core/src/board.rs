//! Board-specific configuration given by the hypervisor binary.

use crate::memmap::GuestPhysicalAddress;

/// Board-specific configuration.
pub struct Board {
    /// Guest kernel image (ELF).
    pub guest_kernel: &'static [u8],
    /// Guest physical address where the guest kernel is loaded.
    pub guest_kernel_base: GuestPhysicalAddress,
    /// Device tree blobs passed to the guest, indexed by hart id.
    pub guest_dtbs: &'static [&'static [u8]],
    /// Guest physical address where the guest device tree is placed.
    pub guest_dtb_base: GuestPhysicalAddress,
    /// Guest initrd image. Empty if the guest does not use an initrd.
    pub guest_initrd: &'static [u8],
}
