pub struct Uart {
    base_addr: u64,
}

impl Uart {
    pub fn new(uart_addr: u64) -> Self {
        Uart {
            base_addr: uart_addr,
        }
    }

    pub fn println(&self, string: &str) {
        let uart = self.base_addr as *mut u32;
        unsafe {
            for c in string.chars() {
                while (uart.read_volatile() as i32) < 0 {}
                uart.write_volatile(c as u32);
            }
            uart.write_volatile('\n' as u32);
        }
    }
}
