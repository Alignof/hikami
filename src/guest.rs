/// Handling guest OS.

/// Guest Information
struct Guest {
    /// Guest ID
    guest_id: usize,
}

impl Guest {
    pub fn new(hart_id: usize) -> Self {
        Guest { guest_id: hart_id }
    }
}
