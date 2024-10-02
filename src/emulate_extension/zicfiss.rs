//! Emulation Zicfiss (Shadow Stack)
//! Ref: [https://github.com/riscv/riscv-cfi/releases/download/v1.0/riscv-cfi.pdf](https://github.com/riscv/riscv-cfi/releases/download/v1.0/riscv-cfi.pdf)

use raki::ZicfissOpcode;

/// Emulate Zicfiss instruction.
pub fn instruction(opc: ZicfissOpcode) {
    match opc {
        ZicfissOpcode::SSPUSH | ZicfissOpcode::C_SSPUSH => todo!(),
        ZicfissOpcode::SSPOPCHK | ZicfissOpcode::C_SSPOPCHK => todo!(),
        ZicfissOpcode::SSRDP => todo!(),
        ZicfissOpcode::SSAMOSWAP_W | ZicfissOpcode::SSAMOSWAP_D => todo!(),
    }
}
