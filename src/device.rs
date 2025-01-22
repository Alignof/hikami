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

trait EmulateDevice {
    /// Pass through loading memory
    fn pass_through_loading(dst_addr: HostPhysicalAddress) -> u32 {
        let dst_ptr = dst_addr.raw() as *const u32;
        unsafe { dst_ptr.read_volatile() }
    }

    /// Emulate loading port registers.
    #[allow(clippy::cast_possible_truncation)]
    fn emulate_loading(&self, base_addr: HostPhysicalAddress, dst_addr: HostPhysicalAddress)
        -> u32;

    /// Pass through storing memory
    fn pass_through_storing(dst_addr: HostPhysicalAddress, value: u32) {
        let dst_ptr = dst_addr.raw() as *mut u32;
        unsafe {
            dst_ptr.write_volatile(value);
        }
    }

    /// Emulate storing port registers.
    fn emulate_storing(
        &mut self,
        base_addr: HostPhysicalAddress,
        dst_addr: HostPhysicalAddress,
        value: u32,
    );
}

/// Memory mapped I/O device.
///
/// A struct that implement this trait **must** has `base_addr` and size member.
#[allow(clippy::module_name_repetitions)]
pub trait MmioDevice {
    /// Create self instance.
    /// * `device_tree` - struct Fdt
    /// * `compatibles` - compatible name list
    fn new(device_tree: &Fdt, compatibles: &[&str]) -> Self;
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
            uart: uart::Uart::new(&device_tree, &["ns16550a", "riscv,axi-uart-1.0"]),
            virtio_list: virtio::VirtIoList::new(&device_tree, "/soc/virtio_mmio"),
            initrd: initrd::Initrd::try_new(&device_tree, "/chosen"),
            plic: plic::Plic::new(&device_tree, &["riscv,plic0"]),
            clint: clint::Clint::new(&device_tree, &["sifive,clint0", "riscv,clint0"]),
            rtc: rtc::Rtc::new(&device_tree, &["google,goldfish-rtc"]),
            pci: pci::Pci::new(&device_tree, &["pci-host-ecam-generic"]),
            mmc: axi_sdc::Mmc::try_new(&device_tree, &["riscv,axi-sd-card-1.0"]),
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
