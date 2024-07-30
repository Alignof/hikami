//! SBI Implementation.  
//! Ref: [https://github.com/rustsbi/rustsbi-qemu](https://github.com/rustsbi/rustsbi-qemu)  
//! Document: [https://docs.rs/rustsbi/0.4.0-alpha.1/rustsbi/derive.RustSBI.html](https://docs.rs/rustsbi/0.4.0-alpha.1/rustsbi/derive.RustSBI.html)  

use crate::device::{clint, uart, Device};
use fdt::Fdt;
use rustsbi::RustSBI;

#[derive(RustSBI)]
pub struct Sbi {
    /// Core Local INTerrupt
    #[rustsbi(ipi, timer)]
    clint: clint::Clint,

    /// Universal Asynchronous Receiver Transmitter
    /// For debug console.
    #[rustsbi(console)]
    pub uart: uart::Uart,
}

impl Sbi {
    pub fn new(device_tree: Fdt) -> Self {
        Sbi {
            uart: uart::Uart::new(&device_tree, "/soc/serial"),
            clint: clint::Clint::new(&device_tree, "/soc/clint"),
        }
    }
}
