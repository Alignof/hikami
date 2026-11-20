//! Codes depend on target board
//!
//! e.g.
//! - kernel image
//! - device tree

#[cfg(all(feature = "qemu", feature = "megrez"))]
compile_error!("features `qemu` and `megrez` are mutually exclusive");
#[cfg(not(any(feature = "qemu", feature = "megrez")))]
compile_error!("either `qemu` or `megrez` feature must be enabled");

#[cfg(feature = "qemu")]
mod qemu;

#[cfg(feature = "qemu")]
pub use qemu::*;

#[cfg(feature = "megrez")]
mod megrez;

#[cfg(feature = "megrez")]
pub use megrez::*;
