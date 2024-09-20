//! IOMMU: I/O memory management unit.
//! Ref: [https://github.com/riscv-non-isa/riscv-iommu/releases/download/v1.0.0/riscv-iommu.pdf](https://github.com/riscv-non-isa/riscv-iommu/releases/download/v1.0.0/riscv-iommu.pdf)

use super::{Device, PTE_FLAGS_FOR_DEVICE};
use crate::memmap::{GuestPhysicalAddress, HostPhysicalAddress, MemoryMap};
use fdt::Fdt;

mod constants {
    //! Constants for IOMMU.

    /// A read-only register reporting features supported by the IOMMU.
    pub const REG_CAPABILITIES: usize = 0x0;
    /// Page-based 39-bit virtual addressing is supported.
    pub const FIELD_CAPABILITIES_SV39X4: usize = 17;

    /// Queue entry size
    /// N = 4096 / 16 = 256
    pub const QUEUE_ENTRY_NUM: usize = 256;
    /// Queue ppn
    /// B = log_2(N) = 8
    pub const QUEUE_PPN: usize = 8;

    /// Command-queue base
    pub const REG_CQB: usize = 0x18;
    /// Holds the number of entries in command-queue as a log to base 2 minus 1.
    pub const FIELD_CQB_LOG2SZ: usize = 0;
    /// Holds the PPN of the root page of the in-memory command-queue used by software to queue commands to the IOMMU.
    pub const FIELD_CQB_PPN: usize = 10;
    /// Command-queue tail
    pub const REG_CQT: usize = 0x24;
    /// Command-queue CSR
    pub const REG_CQCSR: usize = 0x48;
    /// The command-queue is active if cqon is 1.
    pub const FIELD_CQCSR_CQON: usize = 0x10;
}

/// IOMMU: I/O memory management unit.
#[derive(Debug)]
pub struct IoMmu {
    base_addr: HostPhysicalAddress,
    size: usize,
}

impl Device for IoMmu {
    fn new(device_tree: &Fdt, node_path: &str) -> Self {
        let region = device_tree
            .find_node(node_path)
            .unwrap()
            .reg()
            .unwrap()
            .next()
            .unwrap();
        let base_ptr = region.starting_address as *mut u64;
        let base_addr = HostPhysicalAddress(base_ptr as usize);

        // 6.2. Guidelines for initialization
        // p.88

        // 1. Read the capabilities register to discover the capabilities of the IOMMU.
        let capabilities =
            unsafe { core::ptr::read_volatile(base_ptr.byte_add(constants::REG_CAPABILITIES)) };
        // 2. Stop and report failure if capabilities.version is not supported.
        let capabilities_major_version = (capabilities >> 4) & 0xf;
        assert!(capabilities_major_version >= 1);
        let capabilities_sv39x4_supports =
            (capabilities >> constants::FIELD_CAPABILITIES_SV39X4) & 0x1;
        assert_eq!(capabilities_sv39x4_supports, 1);
        // 3. Read the feature control register (fctl).
        // 3~8. are omitted. (does not needed for this system).
        // 9. The icvec register is used to program an interrupt vector for each interrupt cause.
        // 9~11. are omitted. (does not needed for this system).

        // 12. To program the command queue, first determine the number of entries N needed in the command queue.
        // The number of entries in the command queue must be a power of two.
        // Allocate a N x 16-bytes sized memory buffer that is naturally aligned to the greater of 4-KiB or N x 16-bytes.
        // Let k=log2(N) and B be the physical page number (PPN) of the allocated memory buffer.
        unsafe {
            // CQB.PPN = B, CQB.LOG2SZ-1 = k - 1
            core::ptr::write_volatile(
                base_ptr.byte_add(constants::REG_CQB),
                (constants::QUEUE_PPN << constants::FIELD_CQB_PPN
                    | (constants::QUEUE_ENTRY_NUM - 1) << constants::FIELD_CQB_LOG2SZ)
                    as u64,
            );
            // cqt = 0
            core::ptr::write_volatile(base_ptr.byte_add(constants::REG_CQT), 0);
            // cqcsr.cqen = 1
            let cqcsr_value = core::ptr::read_volatile(base_ptr.byte_add(constants::REG_CQCSR));
            core::ptr::write_volatile(base_ptr.byte_add(constants::REG_CQCSR), cqcsr_value | 1);
            // Poll on cqcsr.cqon until it reads 1
            while base_ptr.byte_add(constants::REG_CQCSR).read_volatile()
                >> constants::FIELD_CQCSR_CQON
                & 0x1
                == 0
            {}
        };

        IoMmu {
            base_addr,
            size: region.size.unwrap(),
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
