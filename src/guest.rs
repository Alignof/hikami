/// Handling guest OS.
use crate::memmap::constant::{
    DRAM_BASE, DRAM_SIZE_PAR_HART, GUEST_TEXT_OFFSET, PA2VA_DRAM_OFFSET,
};

/// Guest Information
struct Guest {
    /// Guest ID
    guest_id: usize,
}

impl Guest {
    pub fn new(hart_id: usize) -> Self {
        Guest { guest_id: hart_id }
    }

    /// Return guest dram space start
    fn dram_base(&self) -> usize {
        DRAM_BASE + DRAM_SIZE_PAR_HART * (self.guest_id + 1)
    }
}
