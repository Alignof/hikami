//! Board-specific configuration given by the hypervisor binary.

/// Board-specific configuration.
pub struct Board {
    /// Guest kernel image (ELF).
    pub guest_kernel: &'static [u8],
    /// Device tree blobs passed to the guest, indexed by hart id.
    pub guest_dtbs: &'static [&'static [u8]],
    /// Guest initrd image. Empty if the guest does not use an initrd.
    pub guest_initrd: &'static [u8],
}
