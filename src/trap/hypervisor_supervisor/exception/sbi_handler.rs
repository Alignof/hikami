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

/// sbi ecall handler for RFENCE Extension (EID: #0x52464E43)
#[allow(clippy::module_name_repetitions, clippy::cast_possible_truncation)]
pub fn sbi_rfnc_handler(func_id: usize, args: &[u64; 5]) -> SbiRet {
    use rustsbi::HartMask;
    use sbi_spec::rfnc::{REMOTE_FENCE_I, REMOTE_SFENCE_VMA, REMOTE_SFENCE_VMA_ASID};
    match func_id {
        REMOTE_FENCE_I => {
            sbi_rt::remote_fence_i(HartMask::from_mask_base(args[0] as usize, args[1] as usize))
        }
        REMOTE_SFENCE_VMA => sbi_rt::remote_sfence_vma(
            HartMask::from_mask_base(args[0] as usize, args[1] as usize),
            args[2] as usize,
            args[3] as usize,
        ),
        REMOTE_SFENCE_VMA_ASID => sbi_rt::remote_sfence_vma_asid(
            HartMask::from_mask_base(args[0] as usize, args[1] as usize),
            args[2] as usize,
            args[3] as usize,
            args[4] as usize,
        ),
        _ => panic!("unsupported fid: {}", func_id),
    }
}
