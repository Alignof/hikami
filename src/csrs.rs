//! Define CSRs that are not in the riscv crate.

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

mod hvip {
    const HVIP: usize = 0x645;
    struct Hvip {
        bits: usize,
    }

    read_csr_as!(Hvip, 0x645);
    write_csr_as!(0x645);
}
