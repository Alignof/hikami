/// Devices data
use crate::memmap::DeviceMemmap;
use crate::HypervisorData;

#[derive(Debug)]
pub struct Devices {
    dtb_addr: usize,
    dtb_size: usize,
    memory_map: DeviceMemmap,
}

impl HypervisorData {
    pub fn init_devices(&mut self, dtb_addr: usize, dtb_size: usize, memory_map: DeviceMemmap) {
        self.devices.replace(Devices {
            dtb_addr,
            dtb_size,
            memory_map,
        });
    }
}
