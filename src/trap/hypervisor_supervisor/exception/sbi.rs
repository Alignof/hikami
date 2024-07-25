use crate::guest;

pub fn sbi_base_handler(func_id: usize, context: &guest::context::Context) -> usize {
    use sbi_spec::base::*;
    match func_id {
        GET_SBI_SPEC_VERSION => sbi_rt::get_sbi_impl_version(),
        GET_SBI_IMPL_ID => sbi_rt::get_sbi_impl_id(),
        GET_SBI_IMPL_VERSION => sbi_rt::get_sbi_impl_version(),
        PROBE_EXTENSION => {
            let extension = sbi_rt::get_sbi_impl_version();
            sbi_rt::probe_extension(extension).raw
        }
        GET_MVENDORID => sbi_rt::get_mvendorid(),
        GET_MIMPID => sbi_rt::get_mimpid(),
        GET_MARCHID => sbi_rt::get_marchid(),
    }
}
