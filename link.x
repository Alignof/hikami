/* Common linker script. `memory.x` is copied from linker_scripts/<board>.x by build.rs. */
INCLUDE memory.x

REGION_ALIAS("REGION_TEXT", FLASH);
REGION_ALIAS("REGION_RODATA", FLASH);
REGION_ALIAS("REGION_DATA", RAM);
REGION_ALIAS("REGION_BSS", RAM);
REGION_ALIAS("REGION_HEAP", RAM_HEAP);
REGION_ALIAS("REGION_STACK", L2_LIM);

_stack_start = ORIGIN(L2_LIM) + LENGTH(L2_LIM);

/* Range occupied by the hypervisor. FLASH must be the lowest region and L2_LIM the highest. */
_hypervisor_start = ORIGIN(FLASH);
_hypervisor_end = ORIGIN(L2_LIM) + LENGTH(L2_LIM);
ASSERT(ORIGIN(BOOT_RAM) >= _hypervisor_start && ORIGIN(BOOT_RAM) + LENGTH(BOOT_RAM) <= _hypervisor_end, "BOOT_RAM is out of the hypervisor range")
ASSERT(ORIGIN(RAM_HEAP) >= _hypervisor_start && ORIGIN(RAM_HEAP) + LENGTH(RAM_HEAP) <= _hypervisor_end, "RAM_HEAP is out of the hypervisor range")
ASSERT(ORIGIN(RAM) >= _hypervisor_start && ORIGIN(RAM) + LENGTH(RAM) <= _hypervisor_end, "RAM is out of the hypervisor range")

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

    .guest_kernel : ALIGN(4K) {
        *(.guest_kernel);
        . = ALIGN(4K);
    } > REGION_DATA

    .guest_dtb : ALIGN(4K) {
        *(.guest_dtb);
        . = ALIGN(4K);
    } > REGION_DATA

    .guest_initrd : ALIGN(4K) {
        *(.guest_initrd);
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
