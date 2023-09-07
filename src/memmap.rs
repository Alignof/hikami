pub mod constant;
pub mod device;
use device::{initrd, plic, uart, virtio, Device};
use fdt::Fdt;

/// Memmap has memory region data of each devices.  
/// Each devices **must** be implemented Device trait.
pub struct Memmap {
    pub uart: uart::Uart,
    pub virtio: Vec<virtio::VirtIO>,
    pub initrd: initrd::Initrd,
    pub plic: plic::Plic,
}

impl Memmap {
    pub fn new(device_tree: Fdt) -> Self {
        Memmap {
            uart: uart::Uart::new(&device_tree, "/soc/serial"),
            virtio: virtio::VirtIO::new_all(&device_tree, "/soc/virtio_mmio"),
            initrd: initrd::Initrd::new(&device_tree, "/chosen"),
            plic: plic::Plic::new(&device_tree, "/soc/plic"),
        }
    }
}
