//! AXI SD Card
//!
//! Ref: [https://github.com/eugene-tarassov/vivado-risc-v/blob/master/patches/fpga-axi-sdc.c](https://github.com/eugene-tarassov/vivado-risc-v/blob/master/patches/fpga-axi-sdc.c)

mod register;

use super::{DeviceEmulateError, DmaHostBuffer, EmulateDevice, MmioDevice, PTE_FLAGS_FOR_DEVICE};
use crate::memmap::page_table::{constants::PAGE_SIZE, g_stage_trans_addr};
use crate::memmap::{GuestPhysicalAddress, HostPhysicalAddress, MemoryMap};
use register::SdcRegisters;

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
    dma_alt_buffer: DmaHostBuffer,
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
                            self.dma_alt_buffer.set_used_len(dma_buffer_size);
                            (*registers_ptr).dma_addres = self.dma_alt_buffer.addr() as u64;

                            // write data to allocated memory if command is `write`
                            if ((command >> 6) & 1) == 1 {
                                self.dma_alt_buffer.guest_to_host(dma_gpa);
                            }
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

                    // write back data to guest memory if command is `read`
                    if self.dma_alt_buffer.is_used() {
                        unsafe {
                            if (((*registers_ptr).command >> 5) & 0x1) == 1 {
                                self.dma_alt_buffer.host_to_guest(self.dma_addr);
                            }

                            self.dma_alt_buffer.clear_used_len();
                        }
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
            dma_alt_buffer: DmaHostBuffer::new(PAGE_SIZE),
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
