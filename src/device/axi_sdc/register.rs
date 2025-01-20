//! AXI SD Card Registers

/// Size of `SdcRegisters`;
pub const REG_FIELD_SIZE: usize = core::mem::size_of::<SdcRegisters>();

/// Register definition of AXI SD Card
///
/// Ref: [https://github.com/eugene-tarassov/vivado-risc-v/blob/d72a439f786b455cc321e2e615d7954a75f9ebde/patches/fpga-axi-sdc.c#L67](https://github.com/eugene-tarassov/vivado-risc-v/blob/d72a439f786b455cc321e2e615d7954a75f9ebde/patches/fpga-axi-sdc.c#L67)
#[repr(C)]
pub struct SdcRegisters {
    _argument: u32,
    _command: u32,
    _response1: u32,
    _response2: u32,
    _response3: u32,
    _response4: u32,
    _data_timeout: u32,
    _control: u32,
    _cmd_timeout: u32,
    _clock_divider: u32,
    _software_reset: u32,
    _power_control: u32,
    _capability: u32,
    _cmd_int_status: u32,
    _cmd_int_enable: u32,
    _dat_int_status: u32,
    _dat_int_enable: u32,
    _block_size: u32,
    _block_count: u32,
    _card_detect: u32,
    _reserved: [u32; 4],
    dma_addres: u64,
}
