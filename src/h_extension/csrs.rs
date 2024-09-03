//! Define CSRs that are not in the [riscv crate](https://crates.io/crates/riscv).
//!
//! The specification referred to "The RISC-V Instruction Set Manual: Volume II Version 20240411".

/// Implement bits for struct
#[macro_export]
macro_rules! impl_bits {
    ($register:ident) => {
        #[allow(dead_code)]
        impl $register {
            pub fn bits(&self) -> usize {
                self.bits
            }
        }
    };
}

/// Implement reading CSR method to the struct.
#[macro_export]
macro_rules! read_csr_as {
    ($register:ident, $csr_number:literal) => {
        #[inline]
        #[allow(dead_code)]
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

/// Implement setting to CSR method to the struct.
#[macro_export]
macro_rules! set_csr_as {
    ($csr_number:literal) => {
        #[inline]
        pub fn set(bits: usize) {
            unsafe{
                core::arch::asm!(concat!("csrrs x0, ", stringify!($csr_number), ", {0}"), in(reg) bits);
            }
        }
    };
}

/// Set CSR bit from enum variant.
#[macro_export]
macro_rules! set_csr_from_enum {
    ($enum: ident, $csr_number:literal) => {
        #[inline]
        pub fn set(field: $enum) {
            unsafe{
                core::arch::asm!(concat!("csrrs x0, ", stringify!($csr_number), ", {0}"), in(reg) field as usize);
            }
        }
    };
}

/// Clear CSR bit from enum variant.
#[macro_export]
macro_rules! clear_csr_from_enum {
    ($enum: ident, $csr_number:literal) => {
        #[inline]
        pub fn clear(field: $enum) {
            unsafe{
                core::arch::asm!(concat!("csrrc x0, ", stringify!($csr_number), ", {0}"), in(reg) field as usize);
            }
        }
    };
}

/// VS-level interrupt kind.
pub enum VsInterruptKind {
    /// VS-level external interrupts (bit 10)
    External = 0b100_0000_0000,
    /// VS-level timer interrupts (bit 6)
    Timer = 0b100_0000,
    /// VS-level software interrupts (bit 2)
    Software = 0b100,
}

pub mod vstvec {
    //! Virtual supervisor trap handler base address.
    #![allow(dead_code)]

    const VSTVEC: usize = 0x205;
    pub struct Vstvec {
        bits: usize,
    }

    impl_bits!(Vstvec);

    read_csr_as!(Vstvec, 0x205);
    write_csr_as!(0x205);
}

pub mod vsip {
    //! Virtual supervisor interrupt pending.
    #![allow(dead_code)]

    const VSIP: usize = 0x244;
    pub struct Vsip {
        bits: usize,
    }

    read_csr_as!(Vsip, 0x244);
    write_csr_as!(0x244);

    /// set SSIP bit (`SupervisorSoftwareInterruptPending`, 1 bit)
    pub unsafe fn set_ssoft() {
        core::arch::asm!(
            "
            csrs vsip, {bits}
            ",
            bits = in(reg) 0b0010
        );
    }

    /// set STIP bit (`SupervisorTimerInterruptPending`, 5 bit)
    pub unsafe fn set_stimer() {
        core::arch::asm!(
            "
            csrs vsip, {bits}
            ",
            bits = in(reg) 0b0010_0000
        );
    }
}

pub mod vsatp {
    //! Virtual supervisor address translation and protection.
    #![allow(dead_code)]

    const VSATP: usize = 0x280;
    pub struct Vsatp {
        bits: usize,
    }

    read_csr_as!(Vsatp, 0x280);
    write_csr_as!(0x280);
}

pub mod hstatus {
    //! hstatus util functions.
    #![allow(dead_code)]

    const HSTATUS: usize = 0x600;
    pub struct Hstatus {
        bits: usize,
    }

    read_csr_as!(Hstatus, 0x600);
    write_csr_as!(0x600);

    /// set spv bit (Supervisor Previous Virtualization mode, 7 bit)
    pub unsafe fn set_spv() {
        core::arch::asm!(
            "
            csrs hstatus, {bits}
            ",
            bits = in(reg) 0b1000_0000
        );
    }
}

pub mod hedeleg {
    //! Hypervisor exception delegation register.
    #![allow(dead_code)]

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
    #![allow(dead_code)]

    const HIDELEG: usize = 0x603;
    pub struct Hideleg {
        bits: usize,
    }

    read_csr_as!(Hideleg, 0x603);
    write_csr_as!(0x603);
}

pub mod hie {
    //! Hypervisor interrupt-enable register.
    #![allow(dead_code)]
    use super::VsInterruptKind;

    const HIE: usize = 0x604;
    pub struct Hie {
        bits: usize,
    }

    set_csr_from_enum!(VsInterruptKind, 0x604);
}

pub mod hcounteren {
    //! Hypervisor counter enable.
    #![allow(dead_code)]

    const HCOUNTEREN: usize = 0x606;
    pub struct Hcounteren {
        bits: usize,
    }

    set_csr_as!(0x606);
}

pub mod hvip {
    //! Hypervisor virtual interrupt pending.
    #![allow(dead_code)]
    use super::VsInterruptKind;

    const HVIP: usize = 0x645;
    pub struct Hvip {
        bits: usize,
    }

    set_csr_from_enum!(VsInterruptKind, 0x645);
    clear_csr_from_enum!(VsInterruptKind, 0x645);

    read_csr_as!(Hvip, 0x645);
    write_csr_as!(0x645);
}

pub mod hgatp {
    //! Hypervisor guest address translation and protection.
    #![allow(dead_code)]

    const HGATP: usize = 0x680;
    pub struct Hgatp {
        bits: usize,
    }

    /// Translation mode in G-stage.
    #[allow(clippy::module_name_repetitions)]
    pub enum HgatpMode {
        Bare = 0,
        Sv39x4 = 8,
        Sv48x4 = 9,
        Sv57x4 = 10,
    }

    pub fn set(mode: HgatpMode, vmid: usize, ppn: usize) {
        write((0xF & (mode as usize)) << 60 | (0x3FFF & vmid) << 44 | 0x0FFF_FFFF_FFFF & ppn);
    }

    read_csr_as!(Hgatp, 0x680);
    write_csr_as!(0x680);
}

pub mod henvcfg {
    //! Hypervisor environment configuration register.
    #![allow(dead_code)]

    const HENVCFG: usize = 0x60a;
    pub struct Henvcfg {
        bits: usize,
    }

    /// set STCE (63 bit)
    pub fn set_stce() {
        unsafe {
            core::arch::asm!(
                "
                csrs henvcfg, {bits}
                ",
                bits = in(reg) 1u64 << 63
            );
        }
    }

    /// set CBZE (7 bit)
    pub fn set_cbze() {
        unsafe {
            core::arch::asm!(
                "
                csrs henvcfg, {bits}
                ",
                bits = in(reg) 1u64 << 7
            );
        }
    }

    /// set CBCFE (6 bit)
    pub fn set_cbcfe() {
        unsafe {
            core::arch::asm!(
                "
                csrs henvcfg, {bits}
                ",
                bits = in(reg) 1u64 << 6
            );
        }
    }
}
