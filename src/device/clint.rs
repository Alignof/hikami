//! CLINT: *C*ore *L*ocal *Int*errupt

use super::{MmioDevice, PTE_FLAGS_FOR_DEVICE};
use crate::memmap::{constant, GuestPhysicalAddress, HostPhysicalAddress, MemoryMap};
use fdt::Fdt;
use rustsbi::{HartMask, SbiRet};

mod register {
    //! Ref: [https://chromitem-soc.readthedocs.io/en/latest/clint.html](https://chromitem-soc.readthedocs.io/en/latest/clint.html)
    //!
    //! | Register-Name | Offset(hex) | Size(Bits) | Reset(hex) | Description                                                        |
    //! | ------------- | ----------- | ---------- | ---------- | -----------                                                        |
    //! | msip          | 0x0         | 32         | 0x0        | This register generates machine mode software interrupts when set. |
    //! | mtimecmp      | 0x4000      | 64         | 0x0        | This register holds the compare value for the timer.               |
    //! | mtime         | 0xBFF8      | 64         | 0x0        | Provides the current timer value.                                  |

    /// Offset of `MISP` register
    pub const MSIP_OFFSET: usize = 0x0;
    /// Offset of `MTIMECMP` register
    pub const MTIMECMP_OFFSET: usize = 0x4000;
    #[allow(dead_code)]
    /// Offset of `MTIME` register
    pub const MTIME_OFFSET: usize = 0xbff8;
}

#[allow(clippy::doc_markdown)]
/// CLINT: Core Local INTerrupt
/// Local interrupt controller
#[derive(Debug)]
pub struct Clint {
    /// Base address of memory map.
    base_addr: HostPhysicalAddress,
    /// Memory map size.
    size: usize,
}

impl MmioDevice for Clint {
    fn new(device_tree: &Fdt, node_path: &str) -> Self {
        let region = device_tree
            .find_node(node_path)
            .unwrap()
            .reg()
            .unwrap()
            .next()
            .unwrap();

        Clint {
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

/// Ref: [https://github.com/rustsbi/rustsbi-qemu/blob/main/rustsbi-qemu/src/clint.rs](https://github.com/rustsbi/rustsbi-qemu/blob/main/rustsbi-qemu/src/clint.rs)
impl rustsbi::Timer for Clint {
    /// Programs the clock for the next event after `stime_value` time.
    fn set_timer(&self, stime_value: u64) {
        unsafe {
            let hart_id = riscv::register::mhartid::read();
            assert_eq!(hart_id, 0);
            let mtimecmp_ptr = (self.base_addr.raw() + register::MTIMECMP_OFFSET) as *mut u64;
            mtimecmp_ptr.write_volatile(stime_value);
        }
    }
}

impl rustsbi::Ipi for Clint {
    /// Send an inter-processor interrupt to all the harts defined in `hart_mask`.
    fn send_ipi(&self, hart_mask: HartMask) -> SbiRet {
        for i in 0..constant::MAX_HART_NUM {
            // TODO check hsm wheter allow_ipi enabled.
            if hart_mask.has_bit(i) {
                let msip_ptr = (self.base_addr.raw() + register::MSIP_OFFSET) as *mut u64;
                unsafe {
                    let msip_value = msip_ptr.read_volatile();
                    msip_ptr.write_volatile(msip_value | i as u64);
                }
            }
        }
        SbiRet::success(0)
    }
}
