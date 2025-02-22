//! Utility for PCI configuration registers.
//!
//! Ref: [https://www.macnica.co.jp/business/semiconductor/articles/microchip/140352/](https://www.macnica.co.jp/business/semiconductor/articles/microchip/140352/)

/// Field size of Config Space Header
enum FieldSize {
    /// 1 byte
    Byte1,
    /// 2 byte
    Byte2,
    /// 3 byte
    Byte3,
    /// 4 byte
    Byte4,
}

/// Registers in Common configuration Space Header.
///
/// Ref: [https://astralvx.com/storage/2020/11/PCI_Express_Base_4.0_Rev0.3_February19-2014.pdf](https://astralvx.com/storage/2020/11/PCI_Express_Base_4.0_Rev0.3_February19-2014.pdf) p. 578  
/// Ref: [https://osdev.jp/wiki/PCI-Memo](https://osdev.jp/wiki/PCI-Memo)  
/// Ref: [http://oswiki.osask.jp/?PCI](http://oswiki.osask.jp/?PCI)  
#[derive(Clone, Copy)]
#[allow(dead_code)]
pub enum ConfigSpaceHeaderField {
    /// Vender ID
    VenderId = 0x0,
    /// Device ID
    DeviceId = 0x2,
    /// Command
    Command = 0x4,
    /// Status
    Status = 0x6,
    /// Class Code
    ClassCode = 0x9,
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

impl ConfigSpaceHeaderField {
    /// Field size [byte]
    fn field_size(self) -> FieldSize {
        match self {
            ConfigSpaceHeaderField::VenderId
            | ConfigSpaceHeaderField::DeviceId
            | ConfigSpaceHeaderField::Command
            | ConfigSpaceHeaderField::Status => FieldSize::Byte2,
            ConfigSpaceHeaderField::ClassCode => FieldSize::Byte3,
            ConfigSpaceHeaderField::HeaderType => FieldSize::Byte1,
            ConfigSpaceHeaderField::BaseAddressRegister0
            | ConfigSpaceHeaderField::BaseAddressRegister1
            | ConfigSpaceHeaderField::BaseAddressRegister2
            | ConfigSpaceHeaderField::BaseAddressRegister3
            | ConfigSpaceHeaderField::BaseAddressRegister4
            | ConfigSpaceHeaderField::BaseAddressRegister5 => FieldSize::Byte4,
        }
    }
}

/// Get size of BAR.
#[allow(clippy::cast_possible_truncation)]
pub fn get_bar_size(config_reg_base_addr: usize, reg: ConfigSpaceHeaderField) -> u32 {
    let config_reg_addr = config_reg_base_addr + reg as usize;
    match reg {
        ConfigSpaceHeaderField::BaseAddressRegister0
        | ConfigSpaceHeaderField::BaseAddressRegister1
        | ConfigSpaceHeaderField::BaseAddressRegister2
        | ConfigSpaceHeaderField::BaseAddressRegister3
        | ConfigSpaceHeaderField::BaseAddressRegister4
        | ConfigSpaceHeaderField::BaseAddressRegister5 => unsafe {
            let original_value = core::ptr::read_volatile(config_reg_addr as *const u32);
            core::ptr::write_volatile(config_reg_addr as *mut u32, 0xffff_ffff);
            let size_bit_mask =
                core::ptr::read_volatile(config_reg_addr as *const u32) & 0xffff_fff0;
            core::ptr::write_volatile(config_reg_addr as *mut u32, original_value);

            1 << size_bit_mask.trailing_zeros()
        },
        _ => unreachable!("please specify BAR"),
    }
}

/// Read config data from "PCI Configuration Space".
#[allow(clippy::cast_possible_truncation)]
pub fn read_config_register(config_reg_base_addr: usize, reg: ConfigSpaceHeaderField) -> u32 {
    // the register requires 32 bit size access.
    let config_reg_32bit_addr = (config_reg_base_addr + (reg as usize)) & !0b11;
    let offset_byte = (reg as usize) & 0b11;
    let mask = match reg.field_size() {
        FieldSize::Byte1 => 0xff,
        FieldSize::Byte2 => 0xffff,
        FieldSize::Byte3 => 0xff_ffff,
        FieldSize::Byte4 => 0xffff_ffff,
    };

    let read_value = unsafe { core::ptr::read_volatile(config_reg_32bit_addr as *const u32) };
    (read_value >> (offset_byte * 8)) & mask
}

/// Write config data to "PCI Configuration Space".
#[allow(clippy::cast_possible_truncation)]
pub fn write_config_register(config_reg_base_addr: usize, reg: ConfigSpaceHeaderField, data: u32) {
    // the register requires 32 bit size access.
    let config_reg_32bit_addr = (config_reg_base_addr + (reg as usize)) & !0b11;
    let offset_byte = (reg as usize) & 0b11;
    let mask = match reg.field_size() {
        FieldSize::Byte1 => 0xff,
        FieldSize::Byte2 => 0xffff,
        FieldSize::Byte3 => 0xff_ffff,
        FieldSize::Byte4 => 0xffff_ffff,
    };

    let read_value = unsafe { core::ptr::read_volatile(config_reg_32bit_addr as *const u32) };
    let write_value = (read_value & !(mask << (offset_byte * 8))) | (data << (offset_byte * 8));
    unsafe { core::ptr::write_volatile(config_reg_32bit_addr as *mut u32, write_value) };
}
