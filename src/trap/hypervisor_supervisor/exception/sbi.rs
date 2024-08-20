//! Handle VS-mode Ecall exception
//! See [https://github.com/riscv-non-isa/riscv-sbi-doc/releases/download/v2.0/riscv-sbi.pdf](https://github.com/riscv-non-isa/riscv-sbi-doc/releases/download/v2.0/riscv-sbi.pdf)

use sbi_rt::SbiRet;

/// sbi ecall handler for Base Extension (EID: #0x10)
///
/// All functions in the base extension must be supported by all SBI implementations,
/// so there are no error returns defined. (p.13)
#[allow(clippy::module_name_repetitions)]
pub fn sbi_base_handler(func_id: usize) -> SbiRet {
    use sbi_spec::base::{
        GET_MARCHID, GET_MIMPID, GET_MVENDORID, GET_SBI_IMPL_ID, GET_SBI_IMPL_VERSION,
        GET_SBI_SPEC_VERSION, PROBE_EXTENSION,
    };
    let result_value = match func_id {
        GET_SBI_SPEC_VERSION => {
            let spec = sbi_rt::get_spec_version();
            spec.major() << 24 | spec.minor()
        }
        GET_SBI_IMPL_ID => sbi_rt::get_sbi_impl_id(),
        GET_SBI_IMPL_VERSION => sbi_rt::get_sbi_impl_version(),
        PROBE_EXTENSION => sbi_rt::probe_extension(sbi_rt::Base).raw,
        GET_MVENDORID => sbi_rt::get_mvendorid(),
        GET_MIMPID => sbi_rt::get_mimpid(),
        GET_MARCHID => sbi_rt::get_marchid(),
        _ => unreachable!(),
    };

    SbiRet {
        error: 0, // no error returns
        value: result_value,
    }
}
