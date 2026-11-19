//! Devices data

pub mod aclint;
mod axi_sdc;
pub mod clint;
pub mod pci;
pub mod plic;
pub mod uart;
mod virtio;

use crate::memmap::page_table::{PteFlag, constants::PAGE_SIZE};
use crate::memmap::{HostPhysicalAddress, MemoryMap, page_table};

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use fdt::Fdt;
use fdt::standard_nodes::MemoryRegion;

/// Page table for device
const PTE_FLAGS_FOR_DEVICE: [PteFlag; 6] = [
    PteFlag::Dirty,
    PteFlag::Accessed,
    PteFlag::Write,
    PteFlag::Read,
    PteFlag::User,
    PteFlag::Valid,
];

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
    /// * `root_page_table_addr` - root page table address
    /// * `device_tree` - struct Fdt
    /// * `compatibles` - compatible name list
    fn try_new(
        root_page_table_addr: HostPhysicalAddress,
        device_tree: &Fdt,
        compatibles: &[&str],
    ) -> Option<Self>
    where
        Self: Sized;

    /// Create page table
    fn create_page_table(
        root_page_table_addr: HostPhysicalAddress,
        memory_regions: &[MemoryRegion],
        node_name: &str,
    ) {
        for map in memory_regions {
            crate::println!(
                "[Device Map] {}: {:#x}..{:#x}",
                node_name,
                map.starting_address as usize,
                map.starting_address as usize + map.size.unwrap(),
            )
        }
        let memory_maps: Vec<MemoryMap> = memory_regions
            .iter()
            .cloned()
            .map(|mut region| {
                if (region.starting_address as usize).is_multiple_of(PAGE_SIZE) {
                    MemoryMap::from(region)
                } else {
                    region.starting_address =
                        ((region.starting_address as usize) & !(PAGE_SIZE - 1)) as *const u8;
                    MemoryMap::from(region)
                }
            })
            .collect();
        page_table::sv39x4::generate_page_table(root_page_table_addr, &memory_maps);
    }

    /// Invalidate page table
    ///
    /// This method is disabled by default (only use for emulation).
    fn invalidate_page_table(_memory_regions: &[MemoryRegion], node_name: &str) {
        unreachable!("unreachable code: {}::invalidate_page_table", node_name);
    }

    /// Return device tree node name
    fn name(&self) -> &str;
}

/// Other memory mapped divice
///
/// The all devices which aren't managed by the hypervisor is mapped identically.
#[derive(Debug)]
pub struct OtherMmioDevice {
    /// Device tree name
    pub name: String,
    /// Memory maps for memory mapped register.
    pub register_map_regions: Vec<MemoryRegion>,
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

    /// PLIC: Platform-Level Interrupt Controller  
    pub plic: plic::Plic,

    /// clint: Core Local INTerrupt
    pub clint: Option<clint::Clint>,

    /// aclint: Advanced Core Local INTerrupt
    pub aclint: Option<aclint::Aclint>,

    /// PCI: Peripheral Component Interconnect
    pub pci: Option<pci::Pci>,

    /// Axi SD card
    pub axi_sdc: Option<axi_sdc::Mmc>,

    #[allow(dead_code)]
    /// Other mmio devices
    other_mmio_devices: Vec<OtherMmioDevice>,
}

impl Devices {
    /// Constructor for `Devices`.
    ///
    /// # Panics
    /// Panics if UART or PLIC or CLINT are not found in device tree.
    #[must_use]
    pub fn new(root_page_table_addr: HostPhysicalAddress, device_tree: Fdt) -> Self {
        let uart = uart::Uart::try_new(
            root_page_table_addr,
            &device_tree,
            &["ns16550a", "snps,dw-apb-uart"],
        )
        .expect("uart is not found in fdt");
        let virtio_list =
            virtio::VirtIoList::new(root_page_table_addr, &device_tree, "/soc/virtio_mmio");
        let plic = plic::Plic::try_new(
            root_page_table_addr,
            &device_tree,
            &["riscv,plic0", "sifive,plic-1.0.0"],
        )
        .expect("plic is not found in fdt");
        let clint = clint::Clint::try_new(
            root_page_table_addr,
            &device_tree,
            &["sifive,clint0", "riscv,clint0"],
        );
        let aclint = aclint::Aclint::try_new_aclint(
            root_page_table_addr,
            &device_tree,
            &["thead,c900-aclint-mswi"],
            &["thead,c900-aclint-mtimer"],
        );
        let pci = pci::Pci::try_new(
            root_page_table_addr,
            &device_tree,
            &["pci-host-ecam-generic"],
        );
        let axi_sdc = axi_sdc::Mmc::try_new(
            root_page_table_addr,
            &device_tree,
            &["riscv,axi-sd-card-1.0"],
        );

        // mapping other devices except for devices have already mapped.
        let mut exclude_list: Vec<_> = virtio_list.iter().map(|x| x.name()).collect();
        exclude_list.extend(&[
            uart.name(),
            plic.name(),
            clint.as_ref().map(|x| x.name()).unwrap_or(""),
            aclint.as_ref().map(|x| x.mswi.name()).unwrap_or(""),
            aclint.as_ref().map(|x| x.mtimer.name()).unwrap_or(""),
            pci.as_ref().map(|x| x.name()).unwrap_or(""),
            axi_sdc.as_ref().map(|x| x.name()).unwrap_or(""),
        ]);
        let other_mmio_devices = Self::get_other_mmio_devices(&device_tree, &exclude_list);

        Devices {
            uart,
            virtio_list,
            plic,
            clint,
            aclint,
            pci,
            axi_sdc,
            other_mmio_devices,
        }
    }

    fn get_other_mmio_devices(device_tree: &Fdt, exclude_list: &[&str]) -> Vec<OtherMmioDevice> {
        let mut other_devices = Vec::new();
        if let Some(soc) = device_tree.find_node("/soc") {
            for node in soc.children() {
                // skip if it marked as disabled.
                if let Some(status) = node.property("status")
                    && status.as_str() == Some("disabled")
                {
                    continue;
                }

                // skip if it has already mapped.
                if exclude_list.contains(&node.name) {
                    continue;
                }

                // skip if it isn't memory mapped device.
                if node.reg().is_none() {
                    continue;
                }

                let register_map_regions: Vec<MemoryRegion> = node.reg().unwrap().collect();

                other_devices.push(OtherMmioDevice {
                    name: node.name.to_string(),
                    register_map_regions,
                });
            }
        }

        other_devices
    }
}
