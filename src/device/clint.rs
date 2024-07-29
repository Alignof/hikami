//! CLINT: Core Local INTerrupt

use super::Device;
use crate::memmap::page_table::PteFlag;
use crate::memmap::{constant, MemoryMap};
use fdt::Fdt;
use rustsbi::{HartMask, SbiRet};

const DEVICE_FLAGS: [PteFlag; 5] = [
    PteFlag::Dirty,
    PteFlag::Accessed,
    PteFlag::Write,
    PteFlag::Read,
    PteFlag::Valid,
];

/// CLINT: Core Local INTerrupt
/// Local interrupt controller
#[derive(Debug)]
pub struct Clint {
    base_addr: usize,
    size: usize,
}

impl Device for Clint {
    fn new(device_tree: &Fdt, node_path: &str) -> Self {
        let region = device_tree
            .find_node(node_path)
            .unwrap()
            .reg()
            .unwrap()
            .next()
            .unwrap();

        Clint {
            base_addr: region.starting_address as usize,
            size: region.size.unwrap(),
        }
    }

    fn size(&self) -> usize {
        self.size
    }

    fn paddr(&self) -> usize {
        self.base_addr
    }

    fn vaddr(&self) -> usize {
        self.base_addr + constant::PA2VA_DEVICE_OFFSET
    }

    fn memmap(&self) -> MemoryMap {
        MemoryMap::new(
            self.vaddr()..self.vaddr() + self.size(),
            self.paddr()..self.paddr() + self.size(),
            &DEVICE_FLAGS,
        )
    }

    fn identity_memmap(&self) -> MemoryMap {
        MemoryMap::new(
            self.paddr()..self.paddr() + self.size(),
            self.paddr()..self.paddr() + self.size(),
            &DEVICE_FLAGS,
        )
    }
}

/// Ref: [https://github.com/rustsbi/rustsbi-qemu/blob/main/rustsbi-qemu/src/clint.rs](https://github.com/rustsbi/rustsbi-qemu/blob/main/rustsbi-qemu/src/clint.rs)
impl rustsbi::Timer for Clint {
    /// Programs the clock for the next event after `stime_value` time.
    fn set_timer(&self, stime_value: u64) {
        unsafe {
            let hart_id = riscv::register::mhartid::read();
            assert_eq!(hart_id, 0);
            let mtimecmp_addr = (self.base_addr + constant::clint::MTIMECMP_OFFSET) as *mut u64;
            mtimecmp_addr.write_volatile(stime_value);
        }
    }
}

impl rustsbi::Ipi for Clint {
    /// Send an inter-processor interrupt to all the harts defined in `hart_mask`.
    fn send_ipi(&self, hart_mask: HartMask) -> SbiRet {
        for i in 0..constant::MAX_HART_NUM {
            // TODO check hsm wheter allow_ipi enabled.
            if hart_mask.has_bit(i) {
                let msip_addr = (self.base_addr + constant::clint::MSIP_OFFSET) as *mut u64;
                unsafe {
                    let msip_value = msip_addr.read_volatile();
                    msip_addr.write_volatile(msip_value | i as u64);
                }
            }
        }
        SbiRet::success(0)
    }
}
