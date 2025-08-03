//! Devices data

pub mod aclint;
mod axi_sdc;
pub mod clint;
mod initrd;
pub mod pci;
pub mod plic;
mod sdhci;
pub mod uart;
mod virtio;

use crate::memmap::page_table::{PteFlag, constants::PAGE_SIZE, g_stage_trans_addr};
use crate::memmap::{GuestPhysicalAddress, HostPhysicalAddress, MemoryMap, page_table};

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

/// Device Emulation functions.
///
/// It recives trapped address (and value) and emulate load/store.
pub trait EmulateDevice {
    /// Pass through loading memory
    #[must_use]
    fn pass_through_loading(dst_addr: HostPhysicalAddress) -> u32 {
        let dst_ptr = dst_addr.raw() as *const u32;
        unsafe { dst_ptr.read_volatile() }
    }

    /// Emulate loading port registers.
    ///
    /// # Errors
    /// It will return an error if loading failed.
    #[allow(clippy::cast_possible_truncation)]
    fn emulate_loading(&self, dst_addr: HostPhysicalAddress) -> Result<u32, DeviceEmulateError>;

    /// Pass through storing memory
    fn pass_through_storing(dst_addr: HostPhysicalAddress, value: u32) {
        let dst_ptr = dst_addr.raw() as *mut u32;
        unsafe {
            dst_ptr.write_volatile(value);
        }
    }

    /// Emulate storing port registers.
    ///
    /// # Errors
    /// It will return an error if storing failed.
    fn emulate_storing(
        &mut self,
        dst_addr: HostPhysicalAddress,
        value: u32,
    ) -> Result<(), DeviceEmulateError>;
}

/// DMA buffer for device emulation
#[derive(Debug, Clone)]
struct DmaHostBuffer {
    /// DMA buffer.
    buf: Vec<u8>,
    /// actually used size
    used_len: usize,
}
impl DmaHostBuffer {
    /// Create itself.
    #[allow(clippy::uninit_vec)]
    pub fn new(size: usize) -> Self {
        let mut new_heap = Vec::<u8>::with_capacity(size);
        unsafe {
            new_heap.set_len(size);
        }

        DmaHostBuffer {
            buf: new_heap,
            used_len: 0,
        }
    }

    /// Is it used?
    fn is_used(&self) -> bool {
        self.used_len > 0
    }

    /// Set the size of buffer to use
    ///
    /// If new buffer size is greater than current size, extend the its size.
    fn set_used_len(&mut self, new_len: usize) {
        // extend buffer
        if self.buf.len() < new_len {
            self.buf
                .try_reserve(new_len - self.buf.len())
                .expect("extending DMA host buffer failed");
        } else {
            self.buf[new_len..].fill(0);
        }

        self.used_len = new_len;
    }

    /// Clear buffer len
    fn clear_used_len(&mut self) {
        self.used_len = 0;
    }

    /// Return buffer address
    fn addr(&self) -> usize {
        self.buf.as_ptr() as usize
    }

    /// Copy guest buffer data to host buffer.
    ///
    /// It is used in emulating write command.
    #[allow(clippy::similar_names)]
    fn guest_to_host(&mut self, guest_buf_addr: GuestPhysicalAddress) {
        let buf_ptr = self.buf.as_ptr().cast_mut();
        for offset in (0..self.used_len).step_by(PAGE_SIZE) {
            let src_gpa = guest_buf_addr + offset;
            let src_hpa =
                g_stage_trans_addr(src_gpa).expect("failed translation of data base address");

            unsafe {
                core::ptr::copy(
                    src_hpa.raw() as *const u8,
                    buf_ptr.add(offset),
                    if offset + PAGE_SIZE < self.used_len {
                        PAGE_SIZE
                    } else {
                        self.used_len - offset
                    },
                );
            }
        }
    }

    /// Copy guest buffer data to host buffer.
    ///
    /// It is used in emulating read command.
    #[allow(clippy::similar_names)]
    fn host_to_guest(&mut self, guest_buf_addr: GuestPhysicalAddress) {
        let buf_ptr = self.buf.as_ptr().cast_mut();
        for offset in (0..self.used_len).step_by(PAGE_SIZE) {
            let dst_gpa = guest_buf_addr + offset;
            let dst_hpa =
                g_stage_trans_addr(dst_gpa).expect("failed translation of data base address");

            unsafe {
                core::ptr::copy(
                    buf_ptr.add(offset),
                    dst_hpa.raw() as *mut u8,
                    if offset + PAGE_SIZE < self.used_len {
                        PAGE_SIZE
                    } else {
                        self.used_len - offset
                    },
                );
            }
        }
    }
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
                if region.starting_address as usize % PAGE_SIZE == 0 {
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
    /// Return memory maps between physical to physical (identity map) for crate page table.
    fn memmap(&self) -> Vec<MemoryMap>;
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
    pub clint: Option<clint::Clint>,

    /// aclint: Advanced Core Local INTerrupt
    pub aclint: Option<aclint::Aclint>,

    /// PCI: Peripheral Component Interconnect
    pub pci: Option<pci::Pci>,

    /// Axi SD card
    pub axi_sdc: Option<axi_sdc::Mmc>,

    /// SDHCI: SD Host Controller Interface
    pub sdhci: Option<sdhci::Mmc>,
}

impl Devices {
    /// Constructor for `Devices`.
    ///
    /// # Panics
    /// Panics if UART or PLIC or CLINT are not found in device tree.
    #[must_use]
    pub fn new(root_page_table_addr: HostPhysicalAddress, device_tree: Fdt) -> Self {
        Devices {
            uart: uart::Uart::try_new(
                root_page_table_addr,
                &device_tree,
                &["ns16550a", "snps,dw-apb-uart"],
            )
            .expect("uart is not found in fdt"),
            virtio_list: virtio::VirtIoList::new(
                root_page_table_addr,
                &device_tree,
                "/soc/virtio_mmio",
            ),
            initrd: initrd::Initrd::try_new_from_node_path(
                root_page_table_addr,
                &device_tree,
                "/chosen",
            ),
            plic: plic::Plic::try_new(root_page_table_addr, &device_tree, &["riscv,plic0"])
                .expect("plic is not found in fdt"),
            clint: clint::Clint::try_new(
                root_page_table_addr,
                &device_tree,
                &["sifive,clint0", "riscv,clint0"],
            ),
            aclint: aclint::Aclint::try_new_aclint(
                root_page_table_addr,
                &device_tree,
                &["thead,c900-aclint-mswi"],
                &["thead,c900-aclint-mtimer"],
            ),
            pci: pci::Pci::try_new(
                root_page_table_addr,
                &device_tree,
                &["pci-host-ecam-generic"],
            ),
            axi_sdc: axi_sdc::Mmc::try_new(
                root_page_table_addr,
                &device_tree,
                &["riscv,axi-sd-card-1.0"],
            ),
            sdhci: sdhci::Mmc::try_new(
                root_page_table_addr,
                &device_tree,
                &["eswin,emmc-sdhci-5.1"],
            ),
        }
    }
}
