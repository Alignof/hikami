//! IOMMU: I/O memory management unit.
//! Ref: [https://github.com/riscv-non-isa/riscv-iommu/releases/download/v1.0.0/riscv-iommu.pdf](https://github.com/riscv-non-isa/riscv-iommu/releases/download/v1.0.0/riscv-iommu.pdf)

mod register_map;

use super::{Device, PTE_FLAGS_FOR_DEVICE};
use crate::memmap::{GuestPhysicalAddress, HostPhysicalAddress, MemoryMap};
use crate::PageBlock;
use register_map::IoMmuRegisters;

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

    /// Fault-queue base
    pub const REG_FQB: usize = 0x28;
    /// Holds the number of entries in fault-queue as a log to base 2 minus 1.
    pub const FIELD_FQB_LOG2SZ: usize = 0;
    /// Holds the PPN of the root page of the in-memory fault-queue used by software to queue faults to the IOMMU.
    pub const FIELD_FQB_PPN: usize = 10;
    /// Fault-queue tail
    pub const REG_FQT: usize = 0x34;
    /// Fault-queue CSR
    pub const REG_FQCSR: usize = 0x4c;
    /// The fault-queue is active if cqon is 1.
    pub const FIELD_FQCSR_FQON: usize = 0x10;

    /// Page-request-queue base
    pub const REG_PQB: usize = 0x18;
    /// Holds the number of entries in page-request-queue as a log to base 2 minus 1.
    pub const FIELD_PQB_LOG2SZ: usize = 0;
    /// Holds the PPN of the root page of the in-memory page-request-queue used by software to queue page-requests to the IOMMU.
    pub const FIELD_PQB_PPN: usize = 10;
    /// Page-request-queue tail
    pub const REG_PQT: usize = 0x24;
    /// Page-request-queue CSR
    pub const REG_PQCSR: usize = 0x48;
    /// The page-request-queue is active if cqon is 1.
    pub const FIELD_PQCSR_PQON: usize = 0x10;
}

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
        let (major, minor) = registers.capabilities.version();
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
