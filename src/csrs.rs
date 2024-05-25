//! Define CSRs that are not in the riscv crate.

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

mod hvip {
    const HVIP: usize = 0x645;
    struct Hvip {
        bits: usize,
    }

    read_csr_as!(Hvip, 0x645);
}
