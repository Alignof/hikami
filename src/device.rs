//! Devices data

mod initrd;
mod plic;
mod uart;
mod virtio;

use crate::memmap::DeviceMemmap;
use crate::HypervisorData;
use alloc::vec::Vec;

/// Manage devices sush as uart, plic, etc...
///
/// memory_map has memory region data of each devices.  
/// Each devices **must** be implemented Device trait.
#[derive(Debug)]
pub struct Devices {
    dtb_addr: usize,
    dtb_size: usize,
    uart: uart::Uart,
    virtio: Vec<virtio::VirtIO>,
    initrd: initrd::Initrd,
    plic: plic::Plic,
    plic_context: usize,
}

impl Devices {
    /// Get dtb data.
    fn dtb_data(&self) -> (usize, usize) {
        (self.dtb_addr, self.dtb_size)
    }
}

impl HypervisorData {
    /// Set device data.
    ///
    /// It replace None (uninit value) to Some(init_device).
    pub fn init_devices(&mut self, dtb_addr: usize, dtb_size: usize, memory_map: DeviceMemmap) {
        self.devices.replace(Devices {
            dtb_addr,
            dtb_size,
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
