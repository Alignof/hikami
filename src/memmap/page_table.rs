use super::constant::PAGE_SIZE;
use super::Memmap;
use core::iter::zip;
use core::ops::Range;
use core::slice::from_raw_parts_mut;

pub fn generate_page_table(
    table_start_addr: usize,
    memmap: &mut [(Range<usize>, Range<usize>)],
    device_mmap: Option<Memmap>,
) {
    const PTE_SIZE: usize = 8;
    const FIRST_LEVEL_SIZE: usize = 512;
    let first_lv_page_table: &mut [u64] =
        unsafe { from_raw_parts_mut(table_start_addr as *mut u64, FIRST_LEVEL_SIZE * PTE_SIZE) };
    let second_lv_page_table: &mut [u64] = unsafe {
        let second_level_start = table_start_addr + FIRST_LEVEL_SIZE * PTE_SIZE;
        from_raw_parts_mut(second_level_start as *mut u64, FIRST_LEVEL_SIZE * PTE_SIZE)
    };

    for (v_range, p_range) in memmap {
        assert!(v_range.len() == p_range.len());
        for (v_start, p_start) in zip(v_range, p_range).step_by(PAGE_SIZE) {
            assert!(v_start as usize % PAGE_SIZE == 0);
            assert!(p_start as usize % PAGE_SIZE == 0);

            let vpn1 = (v_start >> 30) & 0x3ff;
            if !already_created(first_lv_page_table[vpn1]) {
                first_lv_page_table[vpn1] =
                    ((vpn1 + FIRST_LEVEL_SIZE) << 19) as u64 | PteFlags::Valid as u64;
            }
        }
    }
}
