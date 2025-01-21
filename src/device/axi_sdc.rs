//! AXI SD Card
//!
//! Ref: [https://github.com/eugene-tarassov/vivado-risc-v/blob/master/patches/fpga-axi-sdc.c](https://github.com/eugene-tarassov/vivado-risc-v/blob/master/patches/fpga-axi-sdc.c)

mod register;

use super::{EmulateDevice, MmioDevice, PTE_FLAGS_FOR_DEVICE};
use crate::memmap::page_table::{constants::PAGE_SIZE, g_stage_trans_addr};
use crate::memmap::{GuestPhysicalAddress, HostPhysicalAddress, MemoryMap};
use register::{SdcRegisters, REG_FIELD_SIZE};

use alloc::vec::Vec;
use fdt::Fdt;

#[allow(clippy::doc_markdown)]
/// MMC: Multi Media Card
#[derive(Debug)]
pub struct Mmc {
    /// Base address of memory map.
    base_addr: HostPhysicalAddress,
    /// Memory map size.
    size: usize,
    /// DMA address.
    dma_addr: GuestPhysicalAddress,
    /// DMA alternative buffer
    dma_alt_buffer: Vec<u8>,
}

impl Mmc {
    /// Get MMC data from device tree.
    pub fn try_new(device_tree: &Fdt, node_path: &str) -> Option<Self> {
        let mmc = device_tree.find_node(node_path)?;
        if mmc.name == "riscv,axi-sd-card-1.0" {
            return None;
        }
        let region = mmc.reg().unwrap().next()?;

        Some(Mmc {
            base_addr: HostPhysicalAddress(region.starting_address as usize),
            size: region.size.unwrap(),
            dma_addr: GuestPhysicalAddress(0),
            dma_alt_buffer: Vec::new(),
        })
    }

    /// Emulate loading port registers.
    #[allow(clippy::cast_possible_truncation)]
    fn emulate_loading(
        &self,
        base_addr: HostPhysicalAddress,
        dst_addr: HostPhysicalAddress,
    ) -> u32 {
        let offset = dst_addr.raw() - base_addr.raw();
        match offset {
            0..REG_FIELD_SIZE => todo!(),
            // other registers
            _ => Self::pass_through_loading(dst_addr),
        }
    }

    /// Emulate storing port registers.
    fn emulate_storing(&mut self, dst_addr: HostPhysicalAddress, value: u64) {
        let offset = dst_addr.raw() - self.base_addr.raw();
        match offset {
            // Command
            4 => {
                let registers_ptr = self.base_addr.raw() as *mut SdcRegisters;
                unsafe {
                    let dma_buffer_size =
                        ((*registers_ptr).block_size * (*registers_ptr).block_count) as usize;
                    let dma_gpa = GuestPhysicalAddress((*registers_ptr).dma_addres as usize);
                    self.dma_addr = dma_gpa;

                    if dma_buffer_size <= PAGE_SIZE {
                        // only translation
                        let dma_hpa =
                            g_stage_trans_addr(dma_gpa).expect("failed to translate dma address");
                        (*registers_ptr).dma_addres = dma_hpa.raw() as u64;
                    } else {
                        // pass new buffer
                        let mut new_heap = Vec::<u8>::with_capacity(dma_buffer_size);
                        new_heap.set_len(dma_buffer_size);
                        let new_heap_addr = new_heap.as_ptr() as usize;
                        (*registers_ptr).dma_addres = new_heap_addr as u64;

                        // write data to allocated memory if command is `write`
                        if (value >> 6 & 1) == 1 {
                            let heap_ptr = new_heap.as_ptr().cast_mut();
                            for offset in (0..new_heap.len()).step_by(PAGE_SIZE) {
                                let dst_gpa = dma_gpa + offset;
                                let dst_hpa = g_stage_trans_addr(dst_gpa)
                                    .expect("failed translation of data base address");

                                core::ptr::copy(
                                    dst_hpa.raw() as *const u8,
                                    heap_ptr.add(offset),
                                    if offset + PAGE_SIZE < new_heap.len() {
                                        PAGE_SIZE
                                    } else {
                                        new_heap.len() - offset
                                    },
                                );
                            }
                        }
                        self.dma_alt_buffer = new_heap;
                    }
                }
                Self::pass_through_storing(dst_addr, value as u32)
            }
            // other registers
            _ => Self::pass_through_storing(dst_addr, value as u32),
        }
    }
}

impl EmulateDevice for Mmc {}

impl MmioDevice for Mmc {
    fn new(_device_tree: &Fdt, _node_path: &str) -> Self {
        panic!("use axi_sdc::try_new instead");
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
