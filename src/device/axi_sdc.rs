//! AXI SD Card
//!
//! Ref: [https://github.com/eugene-tarassov/vivado-risc-v/blob/master/patches/fpga-axi-sdc.c](https://github.com/eugene-tarassov/vivado-risc-v/blob/master/patches/fpga-axi-sdc.c)

mod register;

use super::{DeviceEmulateError, EmulateDevice, MmioDevice, PTE_FLAGS_FOR_DEVICE};
use crate::memmap::page_table::{constants::PAGE_SIZE, g_stage_trans_addr};
use crate::memmap::{GuestPhysicalAddress, HostPhysicalAddress, MemoryMap};
use register::SdcRegisters;

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
    /// Is the mmc command being executed now.
    is_transferring: bool,
}

impl EmulateDevice for Mmc {
    /// Emulate loading port registers.
    #[allow(clippy::cast_possible_truncation)]
    fn emulate_loading(&self, dst_addr: HostPhysicalAddress) -> Result<u32, DeviceEmulateError> {
        Ok(Self::pass_through_loading(dst_addr))
    }

    /// Emulate storing port registers.
    fn emulate_storing(
        &mut self,
        dst_addr: HostPhysicalAddress,
        value: u32,
    ) -> Result<(), DeviceEmulateError> {
        let offset = dst_addr.raw() - self.base_addr.raw();
        match offset {
            // Argument
            //
            // Start transfer when write command to `Argument`
            // See: https://github.com/eugene-tarassov/vivado-risc-v/blob/d72a439f786b455cc321e2e615d7954a75f9ebde/sdc/axi_sdc_controller.v#L392
            0 => {
                let registers_ptr = self.base_addr.raw() as *mut SdcRegisters;
                let command = unsafe { ((*registers_ptr).command) as usize };
                let dma_gpa = GuestPhysicalAddress(unsafe { (*registers_ptr).dma_addres } as usize);

                // if dma block count is zero, use response register instead of buffer.
                if ((command >> 5) & 0b11) != 0b00 && dma_gpa != GuestPhysicalAddress(0) {
                    // command with transfer
                    unsafe {
                        let dma_block_count = ((*registers_ptr).block_count + 1) as usize;
                        let dma_block_size = ((*registers_ptr).block_size + 1) as usize;
                        let dma_buffer_size = dma_block_size * dma_block_count;
                        self.dma_addr = dma_gpa;

                        if dma_buffer_size <= PAGE_SIZE {
                            // only translation
                            let dma_hpa = g_stage_trans_addr(dma_gpa)
                                .expect("failed to translate dma address");
                            (*registers_ptr).dma_addres = dma_hpa.raw() as u64;
                        } else {
                            // pass new buffer
                            let mut new_heap = Vec::<u8>::with_capacity(dma_buffer_size);
                            new_heap.set_len(dma_buffer_size);
                            let new_heap_addr = new_heap.as_ptr() as usize;
                            (*registers_ptr).dma_addres = new_heap_addr as u64;

                            // write data to allocated memory if command is `write`
                            if ((command >> 6) & 1) == 1 {
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
                    self.is_transferring = true;
                }
            }
            // Data interrupt status
            //
            // End transfer if write zero to it
            60 => {
                // end transfer
                if value == 0 && self.is_transferring {
                    let registers_ptr = self.base_addr.raw() as *mut SdcRegisters;
                    // restore address
                    unsafe {
                        (*registers_ptr).dma_addres = self.dma_addr.raw() as u64;
                    }

                    if self.dma_alt_buffer.len() > 0 {
                        unsafe {
                            if (*registers_ptr).command >> 5 & 0x1 == 1 {
                                // write back data to guest memory if command is `read`
                                let heap_ptr = self.dma_alt_buffer.as_ptr().cast_mut();
                                for offset in (0..self.dma_alt_buffer.len()).step_by(PAGE_SIZE) {
                                    let dst_gpa = self.dma_addr + offset;
                                    let dst_hpa = g_stage_trans_addr(dst_gpa)
                                        .expect("failed translation of data base address");

                                    core::ptr::copy(
                                        heap_ptr.add(offset),
                                        dst_hpa.raw() as *mut u8,
                                        if offset + PAGE_SIZE < self.dma_alt_buffer.len() {
                                            PAGE_SIZE
                                        } else {
                                            self.dma_alt_buffer.len() - offset
                                        },
                                    );
                                }
                            }
                        }
                        self.dma_alt_buffer.clear();
                    }

                    self.is_transferring = false;
                }
            }
            // other registers
            _ => (),
        }
        Self::pass_through_storing(dst_addr, value);

        Ok(())
    }
}

impl MmioDevice for Mmc {
    fn try_new(device_tree: &Fdt, compatibles: &[&str]) -> Option<Self> {
        let region = device_tree
            .find_compatible(compatibles)?
            .reg()
            .unwrap()
            .next()?;

        Some(Mmc {
            base_addr: HostPhysicalAddress(region.starting_address as usize),
            size: region.size.unwrap(),
            dma_addr: GuestPhysicalAddress(0),
            dma_alt_buffer: Vec::new(),
            is_transferring: false,
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
