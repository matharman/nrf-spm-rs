/* Linker script for the nRF9160 in secure mode */
MEMORY
{
    FLASH                    : ORIGIN = 0x00000000, LENGTH = 32K
    RAM                      : ORIGIN = 0x20000000, LENGTH = 8K
}
