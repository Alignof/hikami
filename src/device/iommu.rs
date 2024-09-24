//! IOMMU: I/O memory management unit.
//! Ref: [https://github.com/riscv-non-isa/riscv-iommu/releases/download/v1.0.0/riscv-iommu.pdf](https://github.com/riscv-non-isa/riscv-iommu/releases/download/v1.0.0/riscv-iommu.pdf)

mod register_map;

use super::{
    pci::{ConfigSpaceRegister, Pci},
    PciDevice,
};
use crate::h_extension::csrs::hgatp;
use crate::memmap::{page_table::constants::PAGE_SIZE, HostPhysicalAddress};
use crate::PageBlock;
use register_map::{IoMmuMode, IoMmuRegisters};

use fdt::Fdt;

/// IOMMU: I/O memory management unit.
#[derive(Debug)]
pub struct IoMmu {
    bus_number: u32,
    device_number: u32,
    function_number: u32,
}

impl IoMmu {
    /// Set page table in IOMMU.
    fn init_page_table(&self, ddt_addr: HostPhysicalAddress) {
        const OFFSET_IOHGATP: usize = 64;
        // set all ddt entry
        for offset in (0..PAGE_SIZE).step_by(256) {
            let tc_addr = ddt_addr + offset;
            let iohgatp_addr = ddt_addr + offset + OFFSET_IOHGATP;

            unsafe {
                core::ptr::write_volatile(tc_addr.0 as *mut u64, 1);
                core::ptr::write_volatile(iohgatp_addr.0 as *mut u64, hgatp::read().bits as u64);
            }
        }
    }
}

impl PciDevice for IoMmu {
    fn new(device_tree: &Fdt, node_path: &str) -> Self {
        let pci_reg = device_tree
            .find_node(node_path)
            .unwrap()
            .raw_reg()
            .unwrap()
            .next()
            .unwrap();
        assert_eq!(pci_reg.address.len(), 12); // 4 bytes * 3
        let pci_first_reg = (pci_reg.address[0] as u32) << 24
            | (pci_reg.address[1] as u32) << 16
            | (pci_reg.address[2] as u32) << 8
            | pci_reg.address[3] as u32;

        IoMmu {
            bus_number: pci_first_reg >> 16 & 0b1111_1111, // 8 bit
            device_number: pci_first_reg >> 11 & 0b1_1111, // 5 bit
            function_number: pci_first_reg >> 8 & 0b111,   // 3 bit
        }
    }

    fn init(&self, pci: &Pci) {
        const IOMMU_REG_ADDR: u32 = 0x4000_0000;
        pci.write_config_register(
            self.bus_number,
            self.device_number,
            self.function_number,
            ConfigSpaceRegister::BaseAddressRegister1,
            IOMMU_REG_ADDR,
        );
        pci.write_config_register(
            self.bus_number,
            self.device_number,
            self.function_number,
            ConfigSpaceRegister::BaseAddressRegister2,
            0x0000_0000,
        );
        pci.write_config_register(
            self.bus_number,
            self.device_number,
            self.function_number,
            ConfigSpaceRegister::Command,
            0b10, // memory space enable
        );
        let registers = IOMMU_REG_ADDR as *mut IoMmuRegisters;
        let registers = unsafe { &mut *registers };

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
        let command_queue_ptr = command_queue.0 as *mut [u8; 0x1000];
        unsafe {
            core::ptr::write_bytes(command_queue_ptr, 0u8, 1);
        }
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
        let fault_queue = PageBlock::alloc();
        let fault_queue_ptr = fault_queue.0 as *mut [u8; 0x1000];
        unsafe {
            core::ptr::write_bytes(fault_queue_ptr, 0u8, 1);
        }
        registers.fqb.set(fault_queue, 4096);
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
        let page_request_queue = PageBlock::alloc();
        let page_request_queue_ptr = page_request_queue.0 as *mut [u8; 0x1000];
        unsafe {
            core::ptr::write_bytes(page_request_queue_ptr, 0u8, 1);
        }
        registers.pqb.set(page_request_queue, 4096);
        // pqt = 0
        registers.pqt.write(0);
        // pqcsr.pqen = 1
        registers.pqcsr.set_pqen();
        // Poll on pqcsr.pqon until it reads 1
        while !registers.pqcsr.pqon() {}

        // 15. To program the DDT pointer, first determine the supported device_id width Dw and the format of the device-context data structure.
        let ddt_addr = PageBlock::alloc();
        let ddt_ptr = ddt_addr.0 as *mut [u8; 0x1000];
        unsafe {
            core::ptr::write_bytes(ddt_ptr, 0u8, 1);
        }
        self.init_page_table(ddt_addr);
        registers.ddtp.set(IoMmuMode::Lv1, ddt_addr);
    }
}
