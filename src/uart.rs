pub struct Uart {
    base_addr: u64,
}

impl Uart {
    pub fn new(uart_addr: u64) -> Self {
        Uart {
            base_addr: uart_addr,
        }
    }
}
