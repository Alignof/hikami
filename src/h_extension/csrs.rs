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
                self.0
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
            let csr_out;
            unsafe {
                core::arch::asm!(concat!("csrrs {0}, ", stringify!($csr_number), ", x0"), out(reg) csr_out);
            }
            $register(csr_out)
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
    External = 1 << 10,
    /// VS-level timer interrupts (bit 6)
    Timer = 1 << 6,
    /// VS-level software interrupts (bit 2)
    Software = 1 << 2,
}

pub mod vstvec {
    //! Virtual supervisor trap handler base address.
    #![allow(dead_code)]

    /// vstvec register number.
    const VSTVEC: usize = 0x205;
    /// Virtual supervisor trap handler base address.
    pub struct Vstvec(usize);

    impl_bits!(Vstvec);
    read_csr_as!(Vstvec, 0x205);
    write_csr_as!(0x205);
}

pub mod vsip {
    //! Virtual supervisor interrupt pending.
    #![allow(dead_code)]

    /// vsip register number.
    const VSIP: usize = 0x244;
    /// Virtual supervisor interrupt pending.
    pub struct Vsip(usize);

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

    /// vsatp register number.
    const VSATP: usize = 0x280;
    /// Virtual supervisor address translation and protection.
    pub struct Vsatp(usize);

    impl Vsatp {
        /// Current address-translation scheme
        #[inline]
        pub fn mode(&self) -> Mode {
            match self.0 >> 60 {
                0 => Mode::Bare,
                8 => Mode::Sv39,
                9 => Mode::Sv48,
                10 => Mode::Sv57,
                11 => Mode::Sv64,
                _ => unreachable!(),
            }
        }

        /// Physical page number
        #[inline]
        pub fn ppn(&self) -> usize {
            self.0 & 0xFFF_FFFF_FFFF // bits 0-43
        }
    }

    /// Translation mode.
    pub enum Mode {
        Bare = 0,
        Sv39 = 8,
        Sv48 = 9,
        Sv57 = 10,
        Sv64 = 11,
    }

    read_csr_as!(Vsatp, 0x280);
    write_csr_as!(0x280);
}

pub mod hstatus {
    //! hstatus util functions.
    #![allow(dead_code)]

    /// hstatus register number.
    const HSTATUS: usize = 0x600;
    /// hstatus util functions.
    pub struct Hstatus(usize);

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

    /// hedeleg register number.
    const HEDELEG: usize = 0x602;
    /// Hypervisor exception delegation register.
    pub struct Hedeleg(usize);

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

    /// hideleg register number.
    const HIDELEG: usize = 0x603;
    /// Hypervisor interrupt delegation register.
    pub struct Hideleg(usize);

    read_csr_as!(Hideleg, 0x603);
    write_csr_as!(0x603);
}

pub mod hie {
    //! Hypervisor interrupt-enable register.
    #![allow(dead_code)]
    use super::VsInterruptKind;

    /// hie register number.
    const HIE: usize = 0x604;
    /// Hypervisor interrupt-enable register.
    pub struct Hie(usize);

    set_csr_from_enum!(VsInterruptKind, 0x604);
}

pub mod hcounteren {
    //! Hypervisor counter enable.
    #![allow(dead_code)]

    /// hcounteren register number.
    const HCOUNTEREN: usize = 0x606;
    /// Hypervisor counter enable.
    pub struct Hcounteren(usize);

    set_csr_as!(0x606);
}

pub mod hgeie {
    //! Hypervisor guest external interrupt-enable register.
    #![allow(dead_code)]

    /// hcounteren register number.
    const HGEIE: usize = 0x607;
    /// Hypervisor counter enable.
    pub struct Hgeie(usize);

    /// Get the `GEILEN`.
    pub fn get_geilen() -> usize {
        let original_value = read();
        write(0xffff_ffff);
        let set_value = read();
        write(original_value.0);

        (set_value.0 >> 1).trailing_ones() as usize
    }

    write_csr_as!(0x607);
    read_csr_as!(Hgeie, 0x607);
    set_csr_as!(0x607);
}

pub mod henvcfg {
    //! Hypervisor environment configuration register.
    #![allow(dead_code)]

    /// henvcfg register number.
    const HENVCFG: usize = 0x60a;
    /// Hypervisor environment configuration register.
    pub struct Henvcfg(usize);

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

    /// set CDE (60 bit)
    pub fn set_cde() {
        unsafe {
            core::arch::asm!(
                "
                csrs henvcfg, {bits}
                ",
                bits = in(reg) 1u64 << 60
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

pub mod hstateen0 {
    //! Hypervisor State Enable 0 Register.
    #![allow(dead_code)]

    /// hstateen0 register number.
    const HSTATEEN0: usize = 0x60c;
    /// Hypervisor State Enable 0 Register.
    pub struct HstateEn0(usize);

    /// Enable all state except `C` bit
    pub fn all_state_set() {
        unsafe {
            core::arch::asm!("csrs hstateen0, {all_set}", all_set = in(reg) u64::MAX);
        }
    }

    /// Clear `ENVCFG` (62 bit)
    pub fn clear_envcfg() {
        unsafe {
            core::arch::asm!("csrc hstateen0, {bits}", bits = in(reg) 1u64 << 62);
        }
    }
}

pub mod htval {
    //! Hypervisor bad guest physical address.
    #![allow(dead_code)]

    /// htval register number.
    const HTVAL: usize = 0x643;
    /// Hypervisor bad guest physical address.
    pub struct Htval(usize);

    impl_bits!(Htval);
    read_csr_as!(Htval, 0x643);
}

pub mod hvip {
    //! Hypervisor virtual interrupt pending.
    #![allow(dead_code)]

    use super::VsInterruptKind;

    /// hvip register number.
    const HVIP: usize = 0x645;
    /// Hypervisor virtual interrupt pending.
    pub struct Hvip(usize);

    set_csr_from_enum!(VsInterruptKind, 0x645);
    clear_csr_from_enum!(VsInterruptKind, 0x645);

    read_csr_as!(Hvip, 0x645);
    write_csr_as!(0x645);
}

pub mod htinst {
    //! Hypervisor trap instruction (transformed).
    #![allow(dead_code)]

    /// htinst register number.
    const HTINST: usize = 0x64a;
    /// Hypervisor trap instruction (transformed).
    pub struct Htinst(usize);

    impl_bits!(Htinst);
    read_csr_as!(Htinst, 0x64a);
    write_csr_as!(0x64a);
}

pub mod hgatp {
    //! Hypervisor guest address translation and protection.
    #![allow(dead_code)]

    /// hgatp register number.
    const HGATP: usize = 0x680;
    /// Hypervisor guest address translation and protection.
    pub struct Hgatp(usize);

    impl Hgatp {
        /// Return ppn.
        pub fn ppn(&self) -> usize {
            self.0 & 0xfff_ffff_ffff // 44 bit
        }

        /// Return translation mode.
        pub fn mode(&self) -> Mode {
            match (self.0 >> 60) & 0b1111 {
                0 => Mode::Bare,
                8 => Mode::Sv39x4,
                9 => Mode::Sv48x4,
                10 => Mode::Sv57x4,
                _ => unreachable!(),
            }
        }
    }

    /// Translation mode in G-stage.
    #[allow(clippy::module_name_repetitions)]
    pub enum Mode {
        Bare = 0,
        Sv39x4 = 8,
        Sv48x4 = 9,
        Sv57x4 = 10,
    }

    /// Set Hgatp fields.
    pub fn set(mode: Mode, vmid: usize, ppn: usize) {
        write(((0xF & (mode as usize)) << 60) | ((0x3FFF & vmid) << 44) | 0x0FFF_FFFF_FFFF & ppn);
    }

    impl_bits!(Hgatp);
    read_csr_as!(Hgatp, 0x680);
    write_csr_as!(0x680);
}
