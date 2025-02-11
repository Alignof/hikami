//! AXI SD Card Registers

/// Register definition of AXI SD Card
///
/// Ref: [https://github.com/eugene-tarassov/vivado-risc-v/blob/d72a439f786b455cc321e2e615d7954a75f9ebde/patches/fpga-axi-sdc.c#L67](https://github.com/eugene-tarassov/vivado-risc-v/blob/d72a439f786b455cc321e2e615d7954a75f9ebde/patches/fpga-axi-sdc.c#L67)
#[repr(C)]
pub struct SdcRegisters {
    /// Command arguments
    ///
    /// If it is written, command starts.
    _argument: u32,
    /// Command
    pub command: u32,
    /// Response 1
    _response1: u32,
    /// Response 1
    _response2: u32,
    /// Response 1
    _response3: u32,
    /// Response 5
    _response4: u32,
    /// Data transfer timeout
    _data_timeout: u32,
    /// Sdc control
    _control: u32,
    /// Command timeout
    _cmd_timeout: u32,
    /// Clock divider
    _clock_divider: u32,
    /// Software Reset
    _software_reset: u32,
    /// Power control
    _power_control: u32,
    /// Capability
    _capability: u32,
    /// Command interrupt status
    pub cmd_int_status: u32,
    /// Command interrupt enable
    _cmd_int_enable: u32,
    /// Data interrupt status
    _dat_int_status: u32,
    /// Data interrupt enable
    _dat_int_enable: u32,
    /// DMA block size
    pub block_size: u32,
    /// DMA block count
    pub block_count: u32,
    /// Card detect
    _card_detect: u32,
    /// Reserved fields
    _reserved: [u32; 4],
    /// DMA address
    pub dma_addres: u64,
}
