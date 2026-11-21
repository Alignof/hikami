//! ACLINT: *A*dvanced *C*ore *L*ocal *Int*errupt

mod mswi;
mod mtimer;

use super::MmioDevice;

use fdt::Fdt;

#[allow(clippy::doc_markdown)]
/// ACLINT: Advanced Core Local INTerrupt
/// Local interrupt controller
#[derive(Debug)]
pub struct Aclint {
    /// MSWI
    pub mswi: mswi::Mswi,
    /// MTIMER
    pub mtimer: mtimer::Mtimer,
}

impl Aclint {
    pub fn try_new_aclint(
        device_tree: &Fdt,
        mswi_compatibles: &[&str],
        mtimer_compatibles: &[&str],
    ) -> Option<Self> {
        Some(Aclint {
            mswi: mswi::Mswi::try_new(device_tree, mswi_compatibles)?,
            mtimer: mtimer::Mtimer::try_new(device_tree, mtimer_compatibles)?,
        })
    }
}

impl MmioDevice for Aclint {
    fn try_new(_device_tree: &Fdt, _compatibles: &[&str]) -> Option<Self> {
        unreachable!("Use `Aclint::try_new_aclint` instead");
    }

    fn name(&self) -> &str {
        unreachable!("call `Mswi::name` and `Mtimer::name` directly")
    }
}
