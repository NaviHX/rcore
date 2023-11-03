OUTPUT_ARCH(riscv)
ENTRY(_start)

BASE_ADDRESS = 0x80200000;

SECTIONS {
    . = BASE_ADDRESS;
    kernel_start = .;

    text_start = .;
    .text : {
        *(.text.entry)
        . = ALIGN(4K);
        trampoline_start = .;
        *(.text.trampoline)
        . = ALIGN(4K);
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

    .bss : {
        *(.bss.stack)
        bss_start = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
    }
    . = ALIGN(4K);
    bss_end = .;
    kernel_end = .;

    /DISCARD/ : {
        *(.eh_frame)
    }
}
