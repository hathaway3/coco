MEMORY
{
  /* Standard values for RP2350 */
  FLASH : ORIGIN = 0x10000000, LENGTH = 4M
  RAM   : ORIGIN = 0x20000000, LENGTH = 480k  /* Conservative estimate of available RAM (520k total - system overhead) */
  SRAM4 : ORIGIN = 0x20080000, LENGTH = 4K
  SRAM5 : ORIGIN = 0x20081000, LENGTH = 4K
}

SECTIONS {
    /* Boot ROM info: must be in the first 4KB of flash */
    .start_block : ALIGN(4)
    {
        __start_block_addr = .;
        KEEP(*(.start_block));
        KEEP(*(.boot_info));
    } > FLASH
} INSERT AFTER .vector_table;

_stext = ALIGN(ADDR(.start_block) + SIZEOF(.start_block), 16);

SECTIONS {
    .bi_entries : ALIGN(4)
    {
        __bi_entries_start = .;
        KEEP(*(.bi_entries))
        . = ALIGN(4);
        __bi_entries_end = .;
    } > FLASH
} INSERT AFTER .text;

SECTIONS {
    .end_block : ALIGN(4)
    {
        __end_block_addr = .;
        KEEP(*(.end_block));
    } > FLASH
} INSERT AFTER .uninit;

PROVIDE(start_to_end = __end_block_addr - __start_block_addr);
PROVIDE(end_to_start = __start_block_addr - __end_block_addr);

SECTIONS {
    .scratch_x : {
        _scratch_x_start = .;
        *(.scratch_x .scratch_x.*)
        . = ALIGN(4);
        _scratch_x_end = .;
    } > SRAM4 AT > FLASH
    _scratch_x_source = LOADADDR(.scratch_x);

    .scratch_y : {
        _scratch_y_start = .;
        *(.scratch_y .scratch_y.*)
        . = ALIGN(4);
        _scratch_y_end = .;
    } > SRAM5 AT > FLASH
    _scratch_y_source = LOADADDR(.scratch_y);
} INSERT AFTER .rodata;
