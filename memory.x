MEMORY
{
  /* Standard values for RP2350 */
  FLASH : ORIGIN = 0x10000000, LENGTH = 4M
  RAM   : ORIGIN = 0x20000000, LENGTH = 520k
}

SECTIONS {
    .bi_entries :
    {
        __bi_entries_start = .;
        KEEP(*(.bi_entries))
        __bi_entries_end = .;
    } > FLASH
}
INSERT AFTER .text;
