//! Devices data

mod axi_sdc;
pub mod clint;
mod initrd;
mod pci;
pub mod plic;
mod rtc;
pub mod uart;
mod virtio;

use crate::memmap::page_table::PteFlag;
use crate::memmap::{page_table, HostPhysicalAddress, MemoryMap};
use alloc::vec::Vec;
use fdt::Fdt;

/// Page table for device
const PTE_FLAGS_FOR_DEVICE: [PteFlag; 4] =
    [PteFlag::Write, PteFlag::Read, PteFlag::User, PteFlag::Valid];

/// Device emulation error.
#[allow(clippy::module_name_repetitions)]
pub enum DeviceEmulateError {
    /// Invalid plic address.
    InvalidAddress,
    /// Context ID is out of range.
    InvalidContextId,
    /// Accessed register is reserved.
    ReservedRegister,
}

/// Memory mapped I/O device.
///
/// A struct that implement this trait **must** has `base_addr` and size member.
#[allow(clippy::module_name_repetitions)]
pub trait MmioDevice {
    /// Create self instance.
    /// * `device_tree` - struct Fdt
    /// * `node_path` - node path in fdt
    fn new(device_tree: &Fdt, node_path: &str) -> Self;
    /// Return size of memory region.
    fn size(&self) -> usize;
    /// Return address of physical memory
    fn paddr(&self) -> HostPhysicalAddress;
    /// Return memory map between physical to physical (identity map) for crate page table.
    fn memmap(&self) -> MemoryMap;
}

/// Manage devices sush as uart, plic, etc...
///
/// `memory_map` has memory region data of each devices.  
/// Each devices **must** be implemented Device trait.
#[derive(Debug)]
#[allow(clippy::doc_markdown)]
pub struct Devices {
    /// UART: Universal Asynchronous Receiver-Transmitter
    pub uart: uart::Uart,

    /// Lists of Virtio.
    pub virtio_list: virtio::VirtIoList,

    /// initrd: INITial RamDisk
    pub initrd: Option<initrd::Initrd>,

    /// PLIC: Platform-Level Interrupt Controller  
    pub plic: plic::Plic,

    /// clint: Core Local INTerrupt
    pub clint: clint::Clint,

    /// RTC: Real Time Clock.
    pub rtc: rtc::Rtc,

    /// PCI: Peripheral Component Interconnect
    pub pci: pci::Pci,

    pub mmc: Option<axi_sdc::Mmc>,
}

impl Devices {
    /// Constructor for `Devices`.
    pub fn new(device_tree: Fdt) -> Self {
        Devices {
            uart: uart::Uart::new(&device_tree, "/soc/serial"),
            virtio_list: virtio::VirtIoList::new(&device_tree, "/soc/virtio_mmio"),
            initrd: initrd::Initrd::try_new(&device_tree, "/chosen"),
            plic: plic::Plic::new(&device_tree, "/soc/plic"),
            clint: clint::Clint::new(&device_tree, "/soc/clint"),
            rtc: rtc::Rtc::new(&device_tree, "/soc/rtc"),
            pci: pci::Pci::new(&device_tree, "/soc/pci"),
            mmc: axi_sdc::Mmc::try_new(&device_tree, "/io-bus/mmc0"),
        }
    }

    /// Identity map for devices.
    pub fn device_mapping_g_stage(&self, page_table_start: HostPhysicalAddress) {
        let memory_map = self.create_device_map();
        page_table::sv39x4::generate_page_table(page_table_start, &memory_map);
    }

    /// Return devices range to crate identity map.  
    /// It does not return `Plic` address to emulate it.
    fn create_device_map(&self) -> Vec<MemoryMap> {
        let mut device_mapping: Vec<MemoryMap> = self
            .virtio_list
            .iter()
            .flat_map(|virt| [virt.memmap()])
            .collect();

        device_mapping.extend_from_slice(&[
            self.uart.memmap(),
            self.plic.memmap(),
            self.clint.memmap(),
            self.pci.memmap(),
            self.rtc.memmap(),
        ]);

        if let Some(initrd) = &self.initrd {
            device_mapping.extend_from_slice(&[initrd.memmap()]);
        }

        device_mapping.extend_from_slice(self.pci.pci_memory_maps());

        device_mapping
    }
}
