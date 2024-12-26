MEMORY
{
  FLASH (rx) : ORIGIN = 0x80200000, LENGTH = 2M
  BOOT_RAM (rw) : ORIGIN = 0x80400000, LENGTH = 6M
  RAM (rwx) : ORIGIN = 0xc1000000, LENGTH = 528M
  L2_LIM (rw) : ORIGIN = 0xe2000000, LENGTH = 8M
}

/*
 * FLASH (TEXT), 0x8020_0000..0x8040_0000
 * BOOT_RAM , 0x8040_0000..0x80a0_0000
 * RAM (DATA, BSS, HEAP), 0x8100_0000..0xa200_0000
 * L2_LIM (STACK), 0xa200_0000..0xa300_0000
 */

REGION_ALIAS("REGION_TEXT", FLASH);
REGION_ALIAS("REGION_RODATA", FLASH);
REGION_ALIAS("REGION_DATA", RAM);
REGION_ALIAS("REGION_BSS", RAM);
REGION_ALIAS("REGION_HEAP", RAM);
REGION_ALIAS("REGION_STACK", L2_LIM);

_stack_start = ORIGIN(L2_LIM) + LENGTH(L2_LIM);
_hv_heap_size = 0x20000000;
_b_stack_size = 0x200000;

/* defined section in hikami */
SECTIONS
{
    .text : {
        *(.text.entry)
        . = ALIGN(4K);
        *(.text .text.*)
    } > REGION_TEXT

    .boot_stack : ALIGN(4K) {
        _bottom_b_stack = .;
        . += _b_stack_size;
        _top_b_stack = .;
    } > BOOT_RAM

    .host_dtb : ALIGN(4K)
    {
        *(.host_dtb);
        . = ALIGN(4K);
    } > REGION_DATA

    .guest_dtb : ALIGN(4K)
    {
        *(.guest_dtb);
        . = ALIGN(4K);
    } > REGION_DATA

    .root_page_table : ALIGN(16K)
    {
        *(.root_page_table);
        . = ALIGN(4K);
    } > REGION_DATA

    .hv_heap (NOLOAD) : ALIGN(1024K) 
    {
        _start_heap = .;
        . += _hv_heap_size;
        _end_heap = .;
    } > REGION_HEAP
}
