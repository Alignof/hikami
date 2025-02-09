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
    /// - `range_phys_hi`: Upper 32-bit data of child addresses.
    ///
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
        pci_addr_space: &PciAddressSpace,
        memory_maps: &mut Vec<MemoryMap>,
    ) -> Self;

    /// Initialize pci device.
    /// * `pci`: struct `Pci`
    fn init(&self, pci_config_space_base_addr: HostPhysicalAddress);
}

#[derive(Debug)]
/// Pci devices
pub struct PciDevices {
    /// IOMMU: I/O memory management unit.
    iommu: Option<iommu::IoMmu>,
    /// SATA: Serial ATA
    pub sata: Option<sata::Sata>,
}

impl PciDevices {
    /// Constructor of `PciDevices`.
    #[allow(clippy::cast_possible_truncation)]
    pub fn new(
        device_tree: &Fdt,
        pci_config_space_base_addr: usize,
        pci_addr_space: &PciAddressSpace,
        memory_maps: &mut Vec<MemoryMap>,
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
                        (class_code >> 16) & 0xff,
                        (class_code >> 8) & 0xff,
                        class_code & 0xff,
                    );

                    if let (1, 6, 1) = (base_class, sub_class, interface) {
                        sata = Some(sata::Sata::new(
                            bdf,
                            vendor_id.into(),
                            device_id.into(),
                            HostPhysicalAddress(pci_config_space_base_addr),
                            pci_addr_space,
                            memory_maps,
                        ));
                    }

                    // skip remain function id if it's not multi function device.
                    if function == 0 && header_type & 0x80 == 0 {
                        break;
                    }
                }
            }
        }

        PciDevices {
            iommu: iommu::IoMmu::new_from_dtb(device_tree, &["riscv,pci-iommu"]),
            sata,
        }
    }
}

/// PCI address space
///
/// Ref: [https://elinux.org/Device_Tree_Usage#PCI_Address_Translation](https://elinux.org/Device_Tree_Usage#PCI_Address_Translation)
#[derive(Debug)]
pub struct PciAddressSpace {
    /// Base address of address space.
    base_addr: HostPhysicalAddress,
    /// Memory space size.
    _size: usize,
}

impl PciAddressSpace {
    /// Constructor of `PciAddressSpace`.
    pub fn new(device_tree: &Fdt, compatibles: &[&str]) -> Self {
        /// Bytes size of u32.
        const BYTES_U32: usize = 4;
        /// Number of bytes in each range chunks.
        /// `BUS_ADDRESS(3)` - `CPU_PHYSICAL(2)` - `SIZE(2)`
        const RANGE_NUM: usize = 7;

        let ranges = device_tree
            .find_compatible(compatibles)
            .unwrap()
            .property("ranges")
            .unwrap()
            .value;

        assert!(ranges.len() % 4 == 0);
        assert!((ranges.len() / 4) % 7 == 0);

        let get_u32 = |range: &[u8], four_bytes_index: usize| {
            let index = four_bytes_index * 4;
            (u32::from(range[index]) << 24)
                | (u32::from(range[index + 1]) << 16)
                | (u32::from(range[index + 2]) << 8)
                | u32::from(range[index + 3])
        };

        let mut base_addr = HostPhysicalAddress(0);
        let mut size = 0;
        for range in ranges.chunks(RANGE_NUM * BYTES_U32) {
            let bus_address = get_u32(range, 0);

            // ignore I/O space map
            // https://elinux.org/Device_Tree_Usage#PCI_Address_Translation
            if (bus_address >> 24) & 0b11 == 0b10 {
                base_addr = HostPhysicalAddress(
                    ((get_u32(range, 3) as usize) << 32) | get_u32(range, 4) as usize,
                );
                size = ((get_u32(range, 5) as usize) << 32) | get_u32(range, 6) as usize;
            }
        }

        PciAddressSpace {
            base_addr,
            _size: size,
        }
    }
}

/// PCI: Peripheral Component Interconnect
/// Local computer bus.
#[derive(Debug)]
#[allow(clippy::struct_field_names)]
pub struct Pci {
    /// Base address of memory map.
    base_addr: HostPhysicalAddress,
    /// Memory map size.
    size: usize,
    /// PCI address space manager
    _pci_addr_space: PciAddressSpace,
    /// Memory maps for pci devices
    memory_maps: Vec<MemoryMap>,
    /// PCI devices
    pub pci_devices: PciDevices,
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
    fn try_new(device_tree: &Fdt, compatibles: &[&str]) -> Option<Self> {
        let region = device_tree
            .find_compatible(compatibles)?
            .reg()
            .unwrap()
            .next()
            .unwrap();

        let mut memory_maps = Vec::new();
        let base_address = region.starting_address as usize;
        let pci_addr_space = PciAddressSpace::new(device_tree, compatibles);
        let pci_devices =
            PciDevices::new(device_tree, base_address, &pci_addr_space, &mut memory_maps);

        Some(Pci {
            base_addr: HostPhysicalAddress(region.starting_address as usize),
            size: region.size.unwrap(),
            _pci_addr_space: pci_addr_space,
            memory_maps,
            pci_devices,
        })
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
