//! Define CSRs that are not in the [riscv crate](https://crates.io/crates/riscv).
//!
//! The specification referred to "The RISC-V Instruction Set Manual: Volume II Version 20240411".

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

pub mod hedeleg {
    //! Hypervisor exception delegation register.
    const HEDELEG: usize = 0x602;
    pub struct Hedeleg {
        bits: usize,
    }

    /// Exception Kind that possible to delegate to lower privileged.
    ///
    /// Ref: The RISC-V Instruction Set Manual: Volume II Version 20240411, p132 Table 29.
    pub enum ExceptionKind {
        /// Instruction address misaligned (bit 0)
        InstructionAddressMissaligned = 0x1,
        /// Breakpoint (bit 3)
        Breakpoint = 0x8,
        /// Environment call from U-mode or VU-mode (bit 8)
        EnvCallFromUorVU = 0x100,
        /// Instruction page fault (bit 12)
        InstructionPageFault = 0x1000,
        /// Load page fault (bit13)
        LoadPageFault = 0x2000,
        /// Store AMO page fault (bit 15)
        StoreAmoPageFault = 0x8000,
    }

    read_csr_as!(Hedeleg, 0x602);
    write_csr_as!(0x602);
}

pub mod hideleg {
    //! Hypervisor interrupt delegation register.
    const HIDELEG: usize = 0x603;
    pub struct Hideleg {
        bits: usize,
    }

    pub enum InterruptKind {
        /// VS-level external interrupts (bit 10)
        Vsei,
        /// VS-level timer interrupts (bit 6)
        Vsti,
        /// VS-level software interrupts (bit 2)
        Vssi,
    }

    read_csr_as!(Hideleg, 0x603);
    write_csr_as!(0x603);
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

pub mod hgatp {
    //! Hypervisor guest address translation and protection.
    const HGATP: usize = 0x680;
    pub struct Hgatp {
        bits: usize,
    }

    #[allow(clippy::module_name_repetitions)]
    pub enum HgatpMode {
        Bare = 0,
        Sv39x4 = 8,
        Sv48x4 = 9,
        Sv57x4 = 10,
    }

    pub fn set(mode: HgatpMode, vmid: usize, ppn: usize) {
        write((mode as usize) << 60 | vmid << 44 | ppn);
    }

    read_csr_as!(Hgatp, 0x680);
    write_csr_as!(0x680);
}
