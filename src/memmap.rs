//! See `memmap/constant` module for specefic memmory map.

pub mod constant;
pub mod device;
pub mod page_table;

use alloc::vec::Vec;
use device::{initrd, plic, uart, virtio, Device};
use fdt::Fdt;

pub struct MemoryMap {
    phys: Range<usize>,
    virt: Range<usize>,
    flags: u8,
}

impl MemoryMap {
    pub fn new(phys: Range<usize>, virt: Range<usize>, flags: &[PteFlag]) -> Self {
        Self {
            phys,
            virt,
            flags: flags.iter().fold(0, |pte_f, f| (pte_f | *f as u8)),
        }
    }
}

/// Memmap has memory region data of each devices.  
/// Each devices **must** be implemented Device trait.
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
}
