//! SBI Implementation.  
//! Ref: [https://github.com/rustsbi/rustsbi-qemu](https://github.com/rustsbi/rustsbi-qemu)  
//! Document: [https://docs.rs/rustsbi/0.4.0-alpha.1/rustsbi/derive.RustSBI.html](https://docs.rs/rustsbi/0.4.0-alpha.1/rustsbi/derive.RustSBI.html)  

mod rfence;

use crate::device::{clint, uart, MmioDevice};
use fdt::Fdt;
use rustsbi::RustSBI;

/// Device reference for `RustSBI`.
#[derive(RustSBI)]
pub struct Sbi {
    /// Core Local INTerrupt
    #[rustsbi(ipi, timer)]
    pub clint: clint::Clint,

    /// Universal Asynchronous Receiver Transmitter
    /// For debug console.
    #[rustsbi(console)]
    pub uart: uart::Uart,

    /// Remote fence
    #[rustsbi(fence)]
    pub rfence: rfence::RemoteFence,
}

impl Sbi {
    pub fn new(device_tree: Fdt) -> Self {
        Sbi {
            uart: uart::Uart::new(&device_tree, "/soc/serial"),
            clint: clint::Clint::new(&device_tree, "/soc/clint"),
            rfence: rfence::RemoteFence,
        }
    }
}
