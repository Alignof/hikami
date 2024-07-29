//! Devices data

mod initrd;
mod plic;
mod uart;
mod virtio;

use crate::HypervisorData;
use alloc::vec::Vec;
use fdt::Fdt;

/// Manage devices sush as uart, plic, etc...
///
/// memory_map has memory region data of each devices.  
/// Each devices **must** be implemented Device trait.
#[derive(Debug)]
pub struct Devices {
    uart: uart::Uart,
    virtio: Vec<virtio::VirtIO>,
    initrd: initrd::Initrd,
    plic: plic::Plic,
    plic_context: usize,
}

impl HypervisorData {
    /// Set device data.
    ///
    /// It replace None (uninit value) to Some(init_device).
    pub fn init_devices(&mut self, device_tree: Fdt) {
        self.devices.replace(Devices {
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
        });
    }
}
