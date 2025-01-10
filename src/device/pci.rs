//! PCI: Peripheral Component Interconnect

// PCI devices
pub mod iommu;
mod sata;

pub mod config_register;

use super::{MmioDevice, PTE_FLAGS_FOR_DEVICE};
use crate::memmap::{GuestPhysicalAddress, HostPhysicalAddress, MemoryMap};
use config_register::{read_config_register, ConfigSpaceHeaderField};

use alloc::vec::Vec;
use fdt::Fdt;

/// Bus - Device - Function
#[derive(Debug)]
pub struct Bdf {
    /// PCI Bus number
    bus: u32,
    /// PCI Device number
    device: u32,
    /// PCI Function number
    function: u32,
}
impl Bdf {
    /// Create BDF from high 32-bit of PCI addresses.
    ///
    /// - range_phys_hi: Upper 32-bit data of child addresses.
    /// Ref: [https://elinux.org/Device_Tree_Usage#PCI_Address_Translation](https://elinux.org/Device_Tree_Usage#PCI_Address_Translation)
    pub fn new(range_phys_hi: u32) -> Self {
        Bdf {
            bus: (range_phys_hi >> 16) & 0b1111_1111, // 8 bit
            device: (range_phys_hi >> 11) & 0b1_1111, // 5 bit
            function: (range_phys_hi >> 8) & 0b111,   // 3 bit
        }
    }

    /// Calculate offset of config space header
    pub fn calc_config_space_header_offset(&self) -> usize {
        ((self.bus & 0b1111_1111) << 20) as usize
            | ((self.device & 0b1_1111) << 15) as usize
            | ((self.function & 0b111) << 12) as usize
    }
}

/// Pci device.
///
/// A struct that implement this trait **must** has `bus`, `device`, `function` number.
#[allow(clippy::module_name_repetitions)]
pub trait PciDevice {
    /// Create self instance.
    fn new(
        bdf: Bdf,
        vendor_id: u32,
        device_id: u32,
        pci_config_space_base_addr: HostPhysicalAddress,
        memory_maps: &mut Vec<MemoryMap>,
    ) -> Self;

    /// Initialize pci device.
    /// * `pci`: struct `Pci`
    fn init(&self, pci_config_space_base_addr: HostPhysicalAddress);
}

#[derive(Debug)]
/// Pci devices
struct PciDevices {
    /// IOMMU: I/O memory management unit.
    iommu: Option<iommu::IoMmu>,
    /// SATA: Serial ATA
    sata: Option<sata::Sata>,
}

impl PciDevices {
    pub fn new(
        device_tree: &Fdt,
        memory_maps: &mut Vec<MemoryMap>,
        pci_config_space_base_addr: usize,
    ) -> Self {
        /// Max PCI bus size.
        const PCI_MAX_BUS: u8 = 255;
        /// Max PCI device size.
        const PCI_MAX_DEVICE: u8 = 31;
        /// Max PCI function size.
        const PCI_MAX_FUNCTION: u8 = 7;

        let mut sata = None;
        for bus in 0..=PCI_MAX_BUS {
            for device in 0..=PCI_MAX_DEVICE {
                for function in 0..=PCI_MAX_FUNCTION {
                    let bdf = Bdf {
                        bus: bus.into(),
                        device: device.into(),
                        function: function.into(),
                    };
                    let config_space_header_addr =
                        pci_config_space_base_addr | bdf.calc_config_space_header_offset();

                    let vendor_id = read_config_register(
                        config_space_header_addr,
                        ConfigSpaceHeaderField::VenderId,
                    ) as u16;
                    // device is disconnected (not a valid device)
                    if vendor_id == 0xFFFF {
                        continue;
                    }

                    let header_type = read_config_register(
                        config_space_header_addr,
                        ConfigSpaceHeaderField::HeaderType,
                    ) as u8;
                    let device_id = read_config_register(
                        config_space_header_addr,
                        ConfigSpaceHeaderField::DeviceId,
                    ) as u16;

                    let class_code = read_config_register(
                        config_space_header_addr,
                        ConfigSpaceHeaderField::ClassCode,
                    );
                    let (base_class, sub_class, interface) = (
                        class_code >> 24 & 0xff,
                        class_code >> 16 & 0xff,
                        class_code >> 8 & 0xff,
                    );

                    match (base_class, sub_class, interface) {
                        (1, 6, 1) => {
                            sata = Some(sata::Sata::new(
                                bdf,
                                vendor_id.into(),
                                device_id.into(),
                                HostPhysicalAddress(pci_config_space_base_addr),
                                memory_maps,
                            ))
                        }
                        _ => (),
                    }

                    // skip remain function id if it's not multi function device.
                    if function == 0 && header_type & 0x80 == 0 {
                        break;
                    }
                }
            }
        }

        PciDevices {
            iommu: iommu::IoMmu::new_from_dtb(&device_tree, "soc/pci/iommu"),
            sata,
        }
    }
}

/// PCI: Peripheral Component Interconnect
/// Local computer bus.
#[derive(Debug)]
pub struct Pci {
    /// Base address of memory map.
    base_addr: HostPhysicalAddress,
    /// Memory map size.
    size: usize,
    /// Memory maps for pci devices
    memory_maps: Vec<MemoryMap>,
    /// PCI devices
    pci_devices: PciDevices,
}

impl Pci {
    /// Return memory maps of Generic PCI host controller
    ///
    /// Ref: [https://www.kernel.org/doc/Documentation/devicetree/bindings/pci/host-generic-pci.txt](https://www.kernel.org/doc/Documentation/devicetree/bindings/pci/host-generic-pci.txt)
    pub fn pci_memory_maps(&self) -> &[MemoryMap] {
        &self.memory_maps
    }

    /// Initialize PCI devices.
    pub fn init_pci_devices(&self) {
        if let Some(iommu) = &self.pci_devices.iommu {
            iommu.init(self.base_addr);
        }
    }
}

impl MmioDevice for Pci {
    fn new(device_tree: &Fdt, node_path: &str) -> Self {
        let region = device_tree
            .find_node(node_path)
            .unwrap()
            .reg()
            .unwrap()
            .next()
            .unwrap();

        let mut memory_maps = Vec::new();
        // TODO: Verify that this process is needed.
        let base_address = region.starting_address as usize;
        let size = region.size.unwrap() as usize;
        memory_maps.push(MemoryMap::new(
            GuestPhysicalAddress(base_address)..GuestPhysicalAddress(base_address) + size,
            HostPhysicalAddress(base_address)..HostPhysicalAddress(base_address) + size,
            &PTE_FLAGS_FOR_DEVICE,
        ));
        let pci_devices = PciDevices::new(device_tree, &mut memory_maps, base_address);

        Pci {
            base_addr: HostPhysicalAddress(region.starting_address as usize),
            size: region.size.unwrap(),
            memory_maps,
            pci_devices,
        }
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
