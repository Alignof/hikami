//! See `memmap/constant` module for specefic memmory map.

pub mod constant;
pub mod device;
pub mod page_table;

use crate::memmap::page_table::PteFlag;
use alloc::vec::Vec;
use core::ops::Range;
use device::{initrd, plic, uart, virtio, Device};
use fdt::Fdt;

#[derive(Clone)]
pub struct MemoryMap {
    virt: Range<usize>,
    phys: Range<usize>,
    flags: u8,
}

impl MemoryMap {
    pub fn new(virt: Range<usize>, phys: Range<usize>, flags: &[PteFlag]) -> Self {
        Self {
            virt,
            phys,
            flags: flags.iter().fold(0, |pte_f, f| (pte_f | *f as u8)),
        }
    }
}

/// Memmap has memory region data of each devices.  
/// Each devices **must** be implemented Device trait.
#[allow(clippy::module_name_repetitions)]
pub struct DeviceMemmap {
    pub uart: uart::Uart,
    pub virtio: Vec<virtio::VirtIO>,
    pub initrd: initrd::Initrd,
    pub plic: plic::Plic,
    pub plic_context: usize,
}

impl DeviceMemmap {
    /// Create Memmap from device tree blob.
    pub fn new(device_tree: Fdt) -> Self {
        DeviceMemmap {
            uart: uart::Uart::new(&device_tree, "/soc/serial"),
            virtio: virtio::VirtIO::new_all(&device_tree, "/soc/virtio_mmio"),
            initrd: initrd::Initrd::new(&device_tree, "/chosen"),
            plic: plic::Plic::new(&device_tree, "/soc/plic"),
            plic_context: device_tree
                .find_node("/cpus/cpu")
                .unwrap()
                .children()
                .next() // interrupt-controller
                .unwrap()
                .property("phandle")
                .unwrap()
                .value[0] as usize,
            /*
            plic_context: device_tree
                .find_node("/cpus/cpu/interrupt-controller")
                .unwrap()
                .property("phandle")
                .unwrap()
                .value[0] as usize,
            */
        }
    }

    pub fn device_mapping(&self, page_table_start: usize) {
        let memory_map = self.create_device_map();
        page_table::sv39::generate_page_table(page_table_start, &memory_map, false);
    }

    fn create_device_map(&self) -> Vec<MemoryMap> {
        let mut device_mapping: Vec<MemoryMap> = self
            .virtio
            .iter()
            .flat_map(|virt| [virt.memmap(), virt.identity_memmap()])
            .collect();

        device_mapping.extend_from_slice(&[
            self.uart.memmap(),
            self.uart.identity_memmap(),
            self.initrd.memmap(),
            self.initrd.identity_memmap(),
            self.plic.memmap(),
            self.plic.identity_memmap(),
        ]);

        device_mapping
    }
}
