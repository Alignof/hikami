//! Define CSRs that are not in the [riscv crate](https://crates.io/crates/riscv).

/// Implement reading CSR method to the struct.
#[macro_export]
macro_rules! read_csr_as {
    ($register:ident, $csr_number:literal) => {
        #[inline]
        pub fn read() -> $register {
            $register {
                bits: {
                    let csr_out;
                    unsafe {
                        core::arch::asm!(concat!("csrrs {0}, ", stringify!($csr_number), ", x0"), out(reg) csr_out);
                    }
                    csr_out
                }
            }
        }
    };
}

/// Implement writing to CSR method to the struct.
#[macro_export]
macro_rules! write_csr_as {
    ($csr_number:literal) => {
        #[inline]
        pub fn write(bits: usize) {
            unsafe{
                core::arch::asm!(concat!("csrrw x0, ", stringify!($csr_number), ", {0}"), in(reg) bits);
            }
        }
    };
}

pub mod vsatp {
    //! Virtual supervisor address translation and protection.
    const VSATP: usize = 0x280;
    pub struct Vsatp {
        bits: usize,
    }

    read_csr_as!(Vsatp, 0x280);
    write_csr_as!(0x280);
}

pub mod hvip {
    //! Hypervisor virtual interrupt pending.
    const HVIP: usize = 0x645;
    pub struct Hvip {
        bits: usize,
    }

    read_csr_as!(Hvip, 0x645);
    write_csr_as!(0x645);
}
