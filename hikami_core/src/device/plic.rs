//! PLIC: Platform-Level Interrupt Controller  
//! ref: [https://github.com/riscv/riscv-plic-spec/releases/download/1.0.0/riscv-plic-1.0.0.pdf](https://github.com/riscv/riscv-plic-spec/releases/download/1.0.0/riscv-plic-1.0.0.pdf)

use super::{DeviceEmulateError, MmioDevice, PTE_FLAGS_FOR_DEVICE};
use crate::h_extension::csrs::{VsInterruptKind, hvip};
use crate::memmap::constant::MAX_HART_NUM;
use crate::memmap::{GuestPhysicalAddress, HostPhysicalAddress, MemoryMap, page_table};

use alloc::vec::Vec;
use fdt::{Fdt, standard_nodes::MemoryRegion};
use riscv::register::sie;

/// Max number of PLIC context.
pub const MAX_CONTEXT_NUM: usize = MAX_HART_NUM * 2;

/// Base offset of context.
const ENABLE_BASE: usize = 0x2000;
/// Context registers region size.
const ENABLE_SIZE_PER_CONTEXT: usize = 0x80;

/// Base offset of context.
const THRESHOLD_CLAIM_BASE: usize = 0x20_0000;
/// Context registers region size.
const THRESHOLD_CLAIM_SIZE_PER_CONTEXT: usize = 0x1000;
/// Claim/complete register offset from `THRESHOLD_CLAIM_BASE` + `THRESHOLD_CLAIM_SIZE_PER_CONTEXT` * `THRESHOLD_CLAIM_SIZE_PER_CONTEXT`.
const CLAIM_OFFSET: usize = 0x4;
/// End of context registers region.
const THRESHOLD_CLAIM_END: usize =
    THRESHOLD_CLAIM_BASE + THRESHOLD_CLAIM_SIZE_PER_CONTEXT * MAX_CONTEXT_NUM;

/// PLIC context ID.
pub struct ContextId(usize);

impl ContextId {
    /// Create new `ContextId` from hart id.
    ///
    /// Each hart has two id for machine and supervisor.
    #[must_use]
    pub fn new(hart_id: usize, is_supervisor: bool) -> Self {
        ContextId(2 * hart_id + usize::from(is_supervisor))
    }

    /// Return raw usize value.
    #[must_use]
    pub fn raw(&self) -> usize {
        self.0
    }
}

/// PLIC: Platform-Level Interrupt Controller  
/// Interrupt controller for global interrupts.
#[derive(Debug)]
pub struct Plic {
    /// Memory maps for memory mapped register.
    register_map_regions: Vec<MemoryRegion>,
    /// Claim complete flags for external interrupts emulation.
    ///
    /// Each bit indicates whether interrupts are claimed in context.
    claim_complete: [u32; MAX_CONTEXT_NUM],
}

impl Plic {
    /// Return base address.
    /// This function assumes that memory mapped register region is first one.
    fn base_addr(&self) -> HostPhysicalAddress {
        HostPhysicalAddress(self.register_map_regions[0].starting_address as usize)
    }

    /// Return first region size
    /// This function assumes that memory mapped register region is first one.
    fn size(&self) -> usize {
        self.register_map_regions[0]
            .size
            .expect("no size plic memory-mapped register region")
    }

    /// Read plic claim/update register and reflect to `claim_complete`.
    pub fn update_claim_complete(&mut self, context_id: &ContextId) {
        let claim_complete_addr = self.base_addr()
            + THRESHOLD_CLAIM_BASE
            + THRESHOLD_CLAIM_SIZE_PER_CONTEXT * context_id.raw()
            + CLAIM_OFFSET;
        let irq = unsafe { core::ptr::read_volatile(claim_complete_addr.raw() as *const u32) };
        self.claim_complete[context_id.raw()] = irq;
    }

    /// Emulate reading plic context register
    fn context_load(&self, hart_id: usize, offset: usize) -> Result<u32, DeviceEmulateError> {
        let guest_context_id = (offset - THRESHOLD_CLAIM_BASE) / THRESHOLD_CLAIM_SIZE_PER_CONTEXT;
        let context_id = hart_id * 2 + guest_context_id % 2;
        let offset_per_context = offset % THRESHOLD_CLAIM_SIZE_PER_CONTEXT;
        match offset_per_context {
            // threshold
            0 => unreachable!("[may be unreachable] plic threshold read"),
            // claim/complete
            4 => {
                if context_id > MAX_CONTEXT_NUM {
                    Err(DeviceEmulateError::InvalidContextId)
                } else {
                    Ok(self.claim_complete[context_id])
                }
            }
            _ => Err(DeviceEmulateError::InvalidAddress),
        }
    }

    /// Emulate reading plic register.
    ///
    /// # Errors
    /// It will return an error if `dst_addr` is out of range.
    pub fn emulate_loading(
        &self,
        hart_id: usize,
        dst_addr: HostPhysicalAddress,
    ) -> Result<u32, DeviceEmulateError> {
        if !(self.base_addr()..self.base_addr() + self.size()).contains(&dst_addr) {
            return Err(DeviceEmulateError::InvalidAddress);
        }

        let offset = dst_addr.raw() - self.base_addr().raw();
        match offset {
            THRESHOLD_CLAIM_BASE..=THRESHOLD_CLAIM_END => self.context_load(hart_id, offset),
            _ => Err(DeviceEmulateError::InvalidAddress),
        }
    }

    /// Emulate storing plic context register.
    ///
    /// # Errors
    /// It will return an error if `dst_addr` is out of range.
    fn context_storing(
        &mut self,
        hart_id: usize,
        guest_hart_id: usize,
        dst_addr: HostPhysicalAddress,
        value: u32,
    ) -> Result<(), DeviceEmulateError> {
        let offset = dst_addr.raw() - self.base_addr().raw();
        let guest_context_id = (offset - THRESHOLD_CLAIM_BASE) / THRESHOLD_CLAIM_SIZE_PER_CONTEXT;
        let context_id = hart_id * 2 + guest_context_id % 2;

        assert_eq!(guest_hart_id, guest_context_id / 2);

        let offset_per_context = offset % THRESHOLD_CLAIM_SIZE_PER_CONTEXT;
        let phys_hart_ptr = self.base_addr()
            + THRESHOLD_CLAIM_BASE
            + context_id * THRESHOLD_CLAIM_SIZE_PER_CONTEXT
            + offset_per_context;
        match offset_per_context {
            // threshold
            0 => {
                let dst_ptr = phys_hart_ptr.raw() as *mut u32;
                unsafe {
                    dst_ptr.write_volatile(value);
                }

                Ok(())
            }
            // claim/complete
            4 => {
                let dst_ptr = phys_hart_ptr.raw() as *mut u32;
                unsafe {
                    if self.claim_complete[context_id] == value {
                        self.claim_complete[context_id] = 0;
                        dst_ptr.write_volatile(value);

                        hvip::clear(VsInterruptKind::External);
                        sie::set_sext();
                    }
                }

                Ok(())
            }
            8 => Err(DeviceEmulateError::ReservedRegister),
            _ => Err(DeviceEmulateError::InvalidAddress),
        }
    }

    /// Emulate storing plic register.
    ///
    /// # Errors
    /// It will return an error if `dst_addr` is out of range.
    pub fn emulate_storing(
        &mut self,
        hart_id: usize,
        guest_hart_id: usize,
        dst_addr: HostPhysicalAddress,
        value: u32,
    ) -> Result<(), DeviceEmulateError> {
        if !(self.base_addr()..self.base_addr() + self.size()).contains(&dst_addr) {
            return Err(DeviceEmulateError::InvalidAddress);
        }

        let offset = dst_addr.raw() - self.base_addr().raw();
        match offset {
            THRESHOLD_CLAIM_BASE..=THRESHOLD_CLAIM_END => {
                self.context_storing(hart_id, guest_hart_id, dst_addr, value)
            }
            _ => Err(DeviceEmulateError::InvalidAddress),
        }
    }
}

impl MmioDevice for Plic {
    #[allow(clippy::cast_ptr_alignment)]
    fn try_new(
        root_page_table_addr: HostPhysicalAddress,
        device_tree: &Fdt,
        compatibles: &[&str],
    ) -> Option<Self> {
        let plic_node = device_tree.find_compatible(compatibles)?;
        let register_map_regions: Vec<MemoryRegion> = plic_node.reg().unwrap().collect();

        Self::create_page_table(root_page_table_addr, &register_map_regions, plic_node.name);

        Some(Plic {
            register_map_regions,
            claim_complete: [0u32; MAX_CONTEXT_NUM],
        })
    }

    fn create_page_table(
        root_page_table_addr: HostPhysicalAddress,
        memory_regions: &[MemoryRegion],
        node_name: &str,
    ) {
        for map in memory_regions {
            crate::println!(
                "[Device Map] {} {:#x}..{:#x}",
                node_name,
                map.starting_address as usize,
                map.starting_address as usize + THRESHOLD_CLAIM_BASE,
            )
        }
        let memory_maps: Vec<MemoryMap> = memory_regions
            .iter()
            .cloned()
            .map(|region| {
                let virt_start = GuestPhysicalAddress(region.starting_address as usize);
                let phys_start = HostPhysicalAddress(region.starting_address as usize);
                MemoryMap::new(
                    virt_start..virt_start + THRESHOLD_CLAIM_BASE,
                    phys_start..phys_start + THRESHOLD_CLAIM_BASE,
                    &PTE_FLAGS_FOR_DEVICE,
                )
            })
            .collect();
        page_table::sv39x4::generate_page_table(root_page_table_addr, &memory_maps);
    }

    fn memmap(&self) -> Vec<MemoryMap> {
        // Pass through 0x0 - 0x20_0000.
        // Disallow 0x20_0000 - for emulation.
        self.register_map_regions
            .clone()
            .into_iter()
            .map(|region| {
                let virt_start = GuestPhysicalAddress(region.starting_address as usize);
                let phys_start = HostPhysicalAddress(region.starting_address as usize);
                MemoryMap::new(
                    virt_start..virt_start + THRESHOLD_CLAIM_BASE,
                    phys_start..phys_start + THRESHOLD_CLAIM_BASE,
                    &PTE_FLAGS_FOR_DEVICE,
                )
            })
            .collect()
    }
}
