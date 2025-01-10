//! Utility for PCI configuration registers.
//!
//! Ref: [https://www.macnica.co.jp/business/semiconductor/articles/microchip/140352/](https://www.macnica.co.jp/business/semiconductor/articles/microchip/140352/)

/// Field size of Config Space Header
enum FieldSize {
    /// 1 byte
    Byte1,
    /// 2 byte
    Byte2,
    /// 4 byte
    Byte4,
}

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
    /// Header type
    HeaderType = 0xd,
    /// Base Address Register 0
    BaseAddressRegister0 = 0x10,
    /// Base Address Register 1
    BaseAddressRegister1 = 0x14,
    /// Base Address Register 2
    BaseAddressRegister2 = 0x18,
    /// Base Address Register 3
    BaseAddressRegister3 = 0x1c,
    /// Base Address Register 4
    BaseAddressRegister4 = 0x20,
    /// Base Address Register 5
    BaseAddressRegister5 = 0x24,
}

impl ConfigSpaceHeaderRegister {
    /// Field size [byte]
    pub fn field_size(&self) -> FieldSize {
        match self {
            ConfigSpaceHeaderRegister::VenderId => FieldSize::Byte2,
            ConfigSpaceHeaderRegister::DeviceId => FieldSize::Byte2,
            ConfigSpaceHeaderRegister::Command => FieldSize::Byte2,
            ConfigSpaceHeaderRegister::Status => FieldSize::Byte2,
            ConfigSpaceHeaderRegister::HeaderType => FieldSize::Byte1,
            ConfigSpaceHeaderRegister::BaseAddressRegister0 => FieldSize::Byte4,
            ConfigSpaceHeaderRegister::BaseAddressRegister1 => FieldSize::Byte4,
            ConfigSpaceHeaderRegister::BaseAddressRegister2 => FieldSize::Byte4,
            ConfigSpaceHeaderRegister::BaseAddressRegister3 => FieldSize::Byte4,
            ConfigSpaceHeaderRegister::BaseAddressRegister4 => FieldSize::Byte4,
            ConfigSpaceHeaderRegister::BaseAddressRegister5 => FieldSize::Byte4,
        }
    }
}

/// Get size of BAR.
#[allow(clippy::cast_possible_truncation)]
pub fn get_bar_size(config_data_reg_addr: usize, reg: ConfigSpaceHeaderRegister) -> u32 {
    match reg {
        ConfigSpaceHeaderRegister::BaseAddressRegister0
        | ConfigSpaceHeaderRegister::BaseAddressRegister1
        | ConfigSpaceHeaderRegister::BaseAddressRegister2
        | ConfigSpaceHeaderRegister::BaseAddressRegister3
        | ConfigSpaceHeaderRegister::BaseAddressRegister4
        | ConfigSpaceHeaderRegister::BaseAddressRegister5 => unsafe {
            let original_value = core::ptr::read_volatile(config_data_reg_addr as *const u32);
            core::ptr::write_volatile(config_data_reg_addr as *mut u32, 0xffff_ffff);
            let size = core::ptr::read_volatile(config_data_reg_addr as *const u32);
            core::ptr::write_volatile(config_data_reg_addr as *mut u32, original_value);

            size
        },
        _ => unreachable!("please specify BAR"),
    }
}
/// Read config data from "PCI Configuration Space".
#[allow(clippy::cast_possible_truncation)]
pub fn read_config_register(config_data_reg_addr: usize, reg: ConfigSpaceHeaderRegister) -> u32 {
    match reg {
        ConfigSpaceHeaderRegister::HeaderType => unsafe {
            u32::from(core::ptr::read_volatile(config_data_reg_addr as *const u8))
        },
        ConfigSpaceHeaderRegister::VenderId
        | ConfigSpaceHeaderRegister::DeviceId
        | ConfigSpaceHeaderRegister::Command
        | ConfigSpaceHeaderRegister::Status => unsafe {
            u32::from(core::ptr::read_volatile(config_data_reg_addr as *const u16))
        },
        ConfigSpaceHeaderRegister::BaseAddressRegister0
        | ConfigSpaceHeaderRegister::BaseAddressRegister1
        | ConfigSpaceHeaderRegister::BaseAddressRegister2
        | ConfigSpaceHeaderRegister::BaseAddressRegister3
        | ConfigSpaceHeaderRegister::BaseAddressRegister4
        | ConfigSpaceHeaderRegister::BaseAddressRegister5 => unsafe {
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
        ConfigSpaceHeaderRegister::HeaderType => unsafe {
            core::ptr::write_volatile(config_data_reg_addr as *mut u8, data as u8)
        },
        ConfigSpaceHeaderRegister::VenderId
        | ConfigSpaceHeaderRegister::DeviceId
        | ConfigSpaceHeaderRegister::Command
        | ConfigSpaceHeaderRegister::Status => unsafe {
            core::ptr::write_volatile(config_data_reg_addr as *mut u16, data as u16);
        },
        ConfigSpaceHeaderRegister::BaseAddressRegister0
        | ConfigSpaceHeaderRegister::BaseAddressRegister1
        | ConfigSpaceHeaderRegister::BaseAddressRegister2
        | ConfigSpaceHeaderRegister::BaseAddressRegister3
        | ConfigSpaceHeaderRegister::BaseAddressRegister4
        | ConfigSpaceHeaderRegister::BaseAddressRegister5 => unsafe {
            core::ptr::write_volatile(config_data_reg_addr as *mut u32, data);
        },
    }
}
