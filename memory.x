MEMORY
{
  FLASH (rx) : ORIGIN = 0x83000000, LENGTH = 2M
  BOOT_RAM (rw) : ORIGIN = 0x83200000, LENGTH = 6M
  RAM_HEAP (rwx) : ORIGIN = 0x84000000, LENGTH = 384M
  RAM (rwx) : ORIGIN = 0x9c000000, LENGTH = 32M
  L2_LIM (rw) : ORIGIN = 0x9e000000, LENGTH = 8M
}

/*
 * FLASH (TEXT), 0x8300_0000..0x8320_0000
 * BOOT_RAM , 0x8320_0000..0x8380_0000
 * RAM_HEAP (HEAP), 0x8400_0000..0x9c00_0000
 * RAM (DATA, BSS), 0x9c00_0000..0x9e00_0000
 * L2_LIM (STACK), 0x9e00_0000..0x9e80_0000
 */

REGION_ALIAS("REGION_TEXT", FLASH);
REGION_ALIAS("REGION_RODATA", FLASH);
REGION_ALIAS("REGION_DATA", RAM);
REGION_ALIAS("REGION_BSS", RAM);
REGION_ALIAS("REGION_HEAP", RAM_HEAP);
REGION_ALIAS("REGION_STACK", L2_LIM);

_stack_start = ORIGIN(L2_LIM) + LENGTH(L2_LIM);
_hv_heap_size = 0x18000000;
_b_stack_size = 0x200000;

/* defined section in hikami */
SECTIONS
{
    .text : {
        *(.text.entry)
        . = ALIGN(4K);
        *(.text .text.*)
    } > REGION_TEXT

    .boot_stack (NOLOAD) : ALIGN(4K) {
        _bottom_b_stack = .;
        . += _b_stack_size;
        _top_b_stack = .;
    } > BOOT_RAM

    .hv_heap (NOLOAD) : ALIGN(1024K) {
        _start_heap = .;
        . += _hv_heap_size;
        _end_heap = .;
    } > REGION_HEAP

    .host_dtb : ALIGN(4K) {
        *(.host_dtb);
        . = ALIGN(4K);
    } > REGION_DATA

    .guest_kernel : ALIGN(4K) {
        *(.guest_kernel);
        . = ALIGN(4K);
    } > REGION_DATA

    .guest_dtb : ALIGN(4K) {
        *(.guest_dtb);
        . = ALIGN(4K);
    } > REGION_DATA

    .root_page_table : ALIGN(16K) {
        *(.root_page_table);
        . = ALIGN(4K);
    } > REGION_DATA

    .bss : ALIGN(4K) {
        _start_bss = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
        . = ALIGN(4K);
        _end_bss = .;
    } > REGION_BSS
}
