use super::constant::PAGE_SIZE;
use super::Memmap;
use core::ops::Range;

pub fn generate_page_table(
    table_start_addr: usize,
    memmap: &[(Range<u64>, Range<u64>)],
    device_mmap: Option<Memmap>,
) {
    for pt_index in 0..1024 {
        let pt_offset = (table_start_addr + pt_index * 8) as *mut usize;
        unsafe {
            pt_offset.write_volatile(match pt_index {
                // 0x0000_0000_1xxx_xxxx or 0x0000_0000_1xxx_xxxx
                0 => (table_start_addr + PAGE_SIZE) >> 2 | 0x01, // v
                // 0 point to 640 PTE(for 0x0000_0000_1000_0000 -> 0x0000_0000_1000_0000)
                640 => 0x0400_0000 | 0xcf, // d, a, x, w, r, v
                // 0xffff_fffc_0xxx_xxxx ..= 0xffff_ffff_8xxx_xxxx
                496..=503 => (pt_index - 496) << 28 | 0xcf, // a, d, x, w, r, v
                // 0x0000_0000_8xxx_xxxx or 0xffff_ffff_cxxx_xxxx
                2 | 511 => (table_start_addr + PAGE_SIZE) >> 2 | 0x01, // v
                // 2 and 511 point to 512 PTE
                512 => 0x2000_0000 | 0xcb, // d, a, x, r, v
                // 2nd level
                513..=1023 => (0x2000_0000 + ((pt_index - 512) << 19)) | 0xc7, // d, a, w, r, v
                _ => 0,
            });
        }
    }
}
