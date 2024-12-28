//! Handle VS-mode Ecall exception  
//! See [https://github.com/riscv-non-isa/riscv-sbi-doc/releases/download/v2.0/riscv-sbi.pdf](https://github.com/riscv-non-isa/riscv-sbi-doc/releases/download/v2.0/riscv-sbi.pdf)

use sbi_rt::SbiRet;
use sbi_rt::{ConfigFlags, StartFlags, StopFlags};

/// SBI re-ecall
///
/// For now, pass all arguments regardless of the actual number of arguments.
fn sbi_call(ext_id: usize, func_id: usize, args: &[u64; 5]) -> SbiRet {
    let (error, value);
    unsafe {
        core::arch::asm!(
            "ecall",
            in("a7") ext_id,
            in("a6") func_id,
            inlateout("a0") args[0] => error,
            inlateout("a1") args[1] => value,
            in("a2") args[2],
            in("a3") args[3],
            in("a4") args[4],
        );
    }
    SbiRet { error, value }
}

/// SBI ecall handler for Base Extension (EID: #0x10)
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

/// Type of flag for SBI PMU extension.
struct PmuFlag(u64);
impl PmuFlag {
    /// Create `PmuFlag` from a register value.
    pub fn new(val: u64) -> Self {
        PmuFlag(0b1111_1111 & val)
    }
}
impl ConfigFlags for PmuFlag {
    #[allow(clippy::cast_possible_truncation)]
    fn raw(&self) -> usize {
        self.0 as usize
    }
}
impl StartFlags for PmuFlag {
    #[allow(clippy::cast_possible_truncation)]
    fn raw(&self) -> usize {
        self.0 as usize
    }
}
impl StopFlags for PmuFlag {
    #[allow(clippy::cast_possible_truncation)]
    fn raw(&self) -> usize {
        self.0 as usize
    }
}

/// SBI ecall handler for PMU Extension (EID: #0x504D55)
#[allow(clippy::cast_possible_truncation)]
pub fn sbi_pmu_handler(func_id: usize, args: &[u64; 5]) -> SbiRet {
    use sbi_spec::pmu::{
        COUNTER_CONFIG_MATCHING, COUNTER_FW_READ, COUNTER_FW_READ_HI, COUNTER_GET_INFO,
        COUNTER_START, COUNTER_STOP, EID_PMU, NUM_COUNTERS, SNAPSHOT_SET_SHMEM,
    };
    match func_id {
        NUM_COUNTERS => SbiRet {
            error: 0,
            value: sbi_rt::pmu_num_counters(),
        },
        COUNTER_GET_INFO => sbi_rt::pmu_counter_get_info(args[0] as usize),
        COUNTER_CONFIG_MATCHING => sbi_rt::pmu_counter_config_matching(
            args[0] as usize,
            args[1] as usize,
            PmuFlag::new(args[2]),
            args[3] as usize,
            args[4],
        ),
        COUNTER_START => sbi_rt::pmu_counter_start(
            args[0] as usize,
            args[1] as usize,
            PmuFlag::new(args[2]),
            args[3],
        ),
        COUNTER_STOP => {
            sbi_rt::pmu_counter_stop(args[0] as usize, args[1] as usize, PmuFlag::new(args[2]))
        }
        COUNTER_FW_READ => sbi_rt::pmu_counter_fw_read(args[0] as usize),
        COUNTER_FW_READ_HI => sbi_rt::pmu_counter_fw_read_hi(args[0] as usize),
        // `sbi_rt::pmu_snapshot_set_shmem` is unimplemented.
        // thus it is called by ecall instruction directly.
        SNAPSHOT_SET_SHMEM => sbi_call(EID_PMU, SNAPSHOT_SET_SHMEM, args),
        _ => panic!("unsupported fid: {}", func_id),
    }
}

/// SBI ecall handler for RFENCE Extension (EID: #0x52464E43)
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

/// FWFT Feature
/// Ref: [https://github.com/riscv-non-isa/riscv-sbi-doc/releases/download/vv3.0-rc1/riscv-sbi.pdf](https://github.com/riscv-non-isa/riscv-sbi-doc/releases/download/vv3.0-rc1/riscv-sbi.pdf) p.78
#[derive(Debug)]
enum FwftFeature {
    /// Control misaligned access exception delegation to supervisor-mode if medeleg is present.
    MisalignedExcDeleg,
    /// Control landing pad support for supervisor-mode.
    LandingPad,
    /// Control shadow stack support for supervisor-mode.
    ShadowStack,
    /// Control double trap support for supervisor-mode.
    DoubleTrap,
    /// Control hardware updating of PTE A/D bits for supervisor-mode.
    PteAdHwUpdating,
    /// Control the pointer masking tag length for supervisor-mode.
    PointerMaskingPmlen,
}

impl TryFrom<usize> for FwftFeature {
    type Error = usize;
    fn try_from(from: usize) -> Result<Self, Self::Error> {
        match from {
            0 => Ok(FwftFeature::MisalignedExcDeleg),
            1 => Ok(FwftFeature::LandingPad),
            2 => Ok(FwftFeature::ShadowStack),
            3 => Ok(FwftFeature::DoubleTrap),
            4 => Ok(FwftFeature::PteAdHwUpdating),
            5 => Ok(FwftFeature::PointerMaskingPmlen),
            _ => Err(from),
        }
    }
}

/// SBI ecall handler for Firmware Features Extension (EID #0x46574654)
///
/// FWFT ecall will be emulated because `sbi_rt` is not supported.
#[allow(clippy::cast_possible_truncation)]
pub fn sbi_fwft_handler(func_id: usize, args: &[u64; 5]) -> SbiRet {
    /// Firmware Features Set (FID #0)
    const FWFT_SET: usize = 0;
    /// Firmware Features Get (FID #1)
    const FWFT_GET: usize = 1;

    let feature = args[0] as usize;

    match func_id {
        FWFT_SET => match FwftFeature::try_from(feature).unwrap() {
            FwftFeature::ShadowStack => {
                // hypervisor does not use shadow stack.
                SbiRet::success(0)
            }
            feat => unimplemented!("unimplemented feature {:?}", feat),
        },
        FWFT_GET => match FwftFeature::try_from(feature).unwrap() {
            FwftFeature::ShadowStack => {
                // hypervisor does not use shadow stack.
                SbiRet::success(0)
            }
            feat => unimplemented!("unimplemented feature {:?}", feat),
        },
        _ => unreachable!(),
    }
}
