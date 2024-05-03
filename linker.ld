OUTPUT_ARCH("riscv")

ENTRY(_start)
EXTERN(abort);

SECTIONS
{
    . = 0x80000000;

    .reset : 
    {
        KEEP(*(.reset.boot));
    }

    /* text: Program code section */
    .text :
    {
        *(.text. text.*);
    }

    /* rodata: Read-only data */
    .rodata :
    {
        *(.rdata .rodata. .rodata.*);
    }

    /* data: Writable data */
    .data :
    {
        *(.data .data.*);
    }

    _bss_start = .;
    .bss :
    {
        *(.bss .bss.*);
    }
}
