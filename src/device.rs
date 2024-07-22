/// Devices data
///
/// TODO: move memory map data from `memmap/device/*`
use crate::memmap::DeviceMemmap;
use crate::HypervisorData;

#[derive(Debug)]
pub struct Devices {
    dtb_addr: usize,
    dtb_size: usize,
    memory_map: DeviceMemmap,
}

impl Devices {
    fn dtb_data(&self) -> (usize, usize) {
        (self.dtb_addr, self.dtb_size)
    }
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
