OUTPUT_FORMAT(elf64-x86-64)
OUTPUT_ARCH(i386:x86-64)
ENTRY(_start)

PHDRS
{
    text PT_LOAD;
    rodata PT_LOAD;
    data PT_LOAD;
}

SECTIONS
{
    . = 0xffffffff80000000;

    .text ALIGN(4K) :
    {
        *(.text .text.*)
    } :text

    .rodata ALIGN(4K) :
    {
        *(.rodata .rodata.*)
        KEEP(*(.requests))
    } :rodata

    .data ALIGN(4K) :
    {
        *(.data .data.*)
        *(.got .got.* .got.plt)
    } :data

    .got : ALIGN(4K)
    {
        *(.got*)
    } :data

    .bss : ALIGN(4K)
    {
        *(COMMON)
        *(.bss*)

    . = ALIGN(64K);
        __bottom_init_kernel_stack = .;
        . += 64K;
        __top_init_kernel_stack = .;

        . = ALIGN(4K);
        _data_end = .;
    } :data

    .requests ALIGN(8) :
    {
        KEEP(*(.requests))
    } :rodata

    .response ALIGN(8) :
    {
        KEEP(*(.response))
    } :rodata

    /DISCARD/ :
    {
        *(.comment)
        *(.eh_frame)
        *(.eh_frame_hdr)
        *(.note.*)
    }
}
