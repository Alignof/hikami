//! AXI SD Card
//!
//! Ref: [https://github.com/eugene-tarassov/vivado-risc-v/blob/master/patches/fpga-axi-sdc.c](https://github.com/eugene-tarassov/vivado-risc-v/blob/master/patches/fpga-axi-sdc.c)

mod register;

use super::{EmulateDevice, MmioDevice, PTE_FLAGS_FOR_DEVICE};
use crate::memmap::{GuestPhysicalAddress, HostPhysicalAddress, MemoryMap};
use fdt::Fdt;

#[allow(clippy::doc_markdown)]
/// MMC: Multi Media Card
#[derive(Debug)]
pub struct Mmc {
    /// Base address of memory map.
    base_addr: HostPhysicalAddress,
    /// Memory map size.
    size: usize,
}

impl Mmc {
    /// Get MMC data from device tree.
    pub fn try_new(device_tree: &Fdt, node_path: &str) -> Option<Self> {
        let mmc = device_tree.find_node(node_path)?;
        if mmc.name == "riscv,axi-sd-card-1.0" {
            return None;
        }
        let region = mmc.reg().unwrap().next()?;

        Some(Mmc {
            base_addr: HostPhysicalAddress(region.starting_address as usize),
            size: region.size.unwrap(),
        })
    }
}

impl EmulateDevice for Mmc {
    /// Emulate loading port registers.
    #[allow(clippy::cast_possible_truncation)]
    fn emulate_loading(
        &self,
        base_addr: HostPhysicalAddress,
        dst_addr: HostPhysicalAddress,
    ) -> u32 {
        let offset = dst_addr.raw() - base_addr.raw();
        match offset {
            // other registers
            _ => Self::pass_through_loading(dst_addr),
        }
    }

    /// Emulate storing port registers.
    fn emulate_storing(
        &mut self,
        base_addr: HostPhysicalAddress,
        dst_addr: HostPhysicalAddress,
        value: u32,
    ) {
        let offset = dst_addr.raw() - base_addr.raw();
        match offset {
            // other registers
            _ => Self::pass_through_storing(dst_addr, value),
        }
    }
}

impl MmioDevice for Mmc {
    fn new(_device_tree: &Fdt, _node_path: &str) -> Self {
        panic!("use axi_sdc::try_new instead");
    }

    fn size(&self) -> usize {
        self.size
    }

    fn paddr(&self) -> HostPhysicalAddress {
        self.base_addr
    }

    fn memmap(&self) -> MemoryMap {
        let vaddr = GuestPhysicalAddress(self.paddr().raw());
        MemoryMap::new(
            vaddr..vaddr + self.size(),
            self.paddr()..self.paddr() + self.size(),
            &PTE_FLAGS_FOR_DEVICE,
        )
    }
}
