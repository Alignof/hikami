#[cfg(feature = "qemu")]
mod qemu;

#[cfg(feature = "qemu")]
pub use qemu::*;

#[cfg(not(feature = "qemu"))]
mod megrez;

#[cfg(not(feature = "qemu"))]
pub use megrez::*;
