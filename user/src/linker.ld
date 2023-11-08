
OUTPUT_ARCH(riscv)
ENTRY(_start)

/* BASE_ADDRESS = 0x80400000; */
/* Get base address from env */
/* Build user application with `BASE_ADDRESS` set */
/* For batch and physics address mode apps, this should be difference for each app. */
/* For apps with paging and elf parsing enabled, you can set the same address e.g 0x10000 */
BASE_ADDRESS = ${BASE_ADDRESS};

SECTIONS {
    . = BASE_ADDRESS;
    kernel_start = .;

    text_start = .;
    .text : {
        *(.text.entry)
        *(.text .text.*)
    }
    . = ALIGN(4K);
    text_end = .;

    rodata_start = .;
    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }
    . = ALIGN(4K);
    rodata_end = .;

    data_start = .;
    .data : {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }
    data_end = .;

    .bss : {
        *(.bss.stack)
        bss_start = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
    }
    . = ALIGN(4K);
    bss_end = .;

    /DISCARD/ : {
        *(.eh_frame)
    }
}
