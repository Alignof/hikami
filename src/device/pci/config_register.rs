/// Utility for PCI configuration registers.
///
/// Ref: [https://www.macnica.co.jp/business/semiconductor/articles/microchip/140352/](https://www.macnica.co.jp/business/semiconductor/articles/microchip/140352/)

/// Registers in Common configuration Space Header.
///
/// Ref: [https://astralvx.com/storage/2020/11/PCI_Express_Base_4.0_Rev0.3_February19-2014.pdf](https://astralvx.com/storage/2020/11/PCI_Express_Base_4.0_Rev0.3_February19-2014.pdf) p. 578  
/// Ref: [https://osdev.jp/wiki/PCI-Memo](https://osdev.jp/wiki/PCI-Memo)  
/// Ref: [http://oswiki.osask.jp/?PCI](http://oswiki.osask.jp/?PCI)  
#[derive(Clone, Copy)]
pub enum ConfigSpaceHeaderRegister {
    /// Vender ID
    VenderId = 0x0,
    /// Device ID
    DeviceId = 0x2,
    /// Command
    Command = 0x4,
    /// Status
    Status = 0x6,
    /// Base Address Register 1
    BaseAddressRegister1 = 0x10,
    /// Base Address Register 2
    BaseAddressRegister2 = 0x14,
}

/// Read config data from "PCI Configuration Space".
#[allow(clippy::cast_possible_truncation)]
pub fn read_config_register(config_data_reg_addr: usize, reg: ConfigSpaceHeaderRegister) -> u32 {
    match reg {
        ConfigSpaceHeaderRegister::VenderId
        | ConfigSpaceHeaderRegister::DeviceId
        | ConfigSpaceHeaderRegister::Command
        | ConfigSpaceHeaderRegister::Status => unsafe {
            u32::from(core::ptr::read_volatile(config_data_reg_addr as *const u16))
        },
        ConfigSpaceHeaderRegister::BaseAddressRegister1
        | ConfigSpaceHeaderRegister::BaseAddressRegister2 => unsafe {
            core::ptr::read_volatile(config_data_reg_addr as *const u32)
        },
    }
}

/// Read config data from "PCI Configuration Space".
#[allow(clippy::cast_possible_truncation)]
pub fn write_config_register(
    config_data_reg_addr: usize,
    reg: ConfigSpaceHeaderRegister,
    data: u32,
) {
    match reg {
        ConfigSpaceHeaderRegister::VenderId
        | ConfigSpaceHeaderRegister::DeviceId
        | ConfigSpaceHeaderRegister::Command
        | ConfigSpaceHeaderRegister::Status => unsafe {
            core::ptr::write_volatile(config_data_reg_addr as *mut u16, data as u16);
        },
        ConfigSpaceHeaderRegister::BaseAddressRegister1
        | ConfigSpaceHeaderRegister::BaseAddressRegister2 => unsafe {
            core::ptr::write_volatile(config_data_reg_addr as *mut u32, data);
        },
    }
}
