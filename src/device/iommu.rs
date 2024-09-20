//! IOMMU: I/O memory management unit.
//! Ref: [https://github.com/riscv-non-isa/riscv-iommu/releases/download/v1.0.0/riscv-iommu.pdf](https://github.com/riscv-non-isa/riscv-iommu/releases/download/v1.0.0/riscv-iommu.pdf)

mod register_map;

use super::{Device, PTE_FLAGS_FOR_DEVICE};
use crate::memmap::{GuestPhysicalAddress, HostPhysicalAddress, MemoryMap};
use crate::PageBlock;
use register_map::IoMmuRegisters;

use fdt::Fdt;

/// For `ddtp.iommu_mode`.
enum IoMmuMode {
    /// No inbound memory transactions are allowed by the IOMMU.
    Off,
    /// No translation or protection. All inbound memory accesses are passed through.
    Bare,
    /// One-level device-directory-table
    Lv1,
    /// Two-level device-directory-table
    Lv2,
    /// Three-level device-directory-table
    Lv3,
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
        let registers = unsafe { &mut *(region.starting_address as *mut IoMmuRegisters) };

        // 6.2. Guidelines for initialization
        // p.88

        // 1. Read the capabilities register to discover the capabilities of the IOMMU.
        // 2. Stop and report failure if capabilities.version is not supported.
        let (major, _minor) = registers.capabilities.version();
        assert!(major >= 1);
        assert!(registers.capabilities.is_sv39x4_supported());

        // 3. Read the feature control register (fctl).
        // 3~8. are omitted. (does not needed for this system).
        // 9. The icvec register is used to program an interrupt vector for each interrupt cause.
        // 9~11. are omitted. (does not needed for this system).

        // 12. To program the command queue, first determine the number of entries N needed in the command queue.
        // The number of entries in the command queue must be a power of two.
        // Allocate a N x 16-bytes sized memory buffer that is naturally aligned to the greater of 4-KiB or N x 16-bytes.
        // Let k=log2(N) and B be the physical page number (PPN) of the allocated memory buffer.
        // CQB.PPN = B, CQB.LOG2SZ-1 = k - 1
        let command_queue = PageBlock::alloc();
        registers.cqb.set(command_queue, 4096);
        // cqt = 0
        registers.cqt.write(0);
        // cqcsr.cqen = 1
        registers.cqcsr.set_cqen();
        // Poll on cqcsr.cqon until it reads 1
        while !registers.cqcsr.cqon() {}

        // 13. To program the fault queue, first determine the number of entries N needed in the fault queue.
        // The number of entries in the fault queue is always a power of two.
        // Allocate a N x 32-bytes sized memory buffer that is naturally aligned to the greater of 4-KiB or N x 32-bytes.
        // Let k=log2(N) and B be the PPN of the allocated memory buffer.
        // FQB.PPN = B, FQB.LOG2SZ-1 = k - 1
        let command_queue = PageBlock::alloc();
        registers.fqb.set(command_queue, 4096);
        // fqt = 0
        registers.fqt.write(0);
        // fqcsr.fqen = 1
        registers.fqcsr.set_fqen();
        // Poll on fqcsr.fqon until it reads 1
        while !registers.fqcsr.fqon() {}

        // 14. To program the page-request queue, first determine the number of entries N needed in the page-request queue.
        // The number of entries in the page-request queue is always a power of two.
        // Allocate a N x 16-bytes sized buffer that is naturally aligned to the greater of 4-KiB or N x 16-bytes.
        // Let k=log2(N) and B be the PPN of the allocated memory buffer.
        // PQB.PPN = B, PQB.LOG2SZ-1 = k - 1
        let command_queue = PageBlock::alloc();
        registers.pqb.set(command_queue, 4096);
        // pqt = 0
        registers.pqt.write(0);
        // pqcsr.pqen = 1
        registers.pqcsr.set_pqen();
        // Poll on pqcsr.pqon until it reads 1
        while !registers.pqcsr.pqon() {}

        IoMmu {
            base_addr: HostPhysicalAddress(region.starting_address as usize),
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