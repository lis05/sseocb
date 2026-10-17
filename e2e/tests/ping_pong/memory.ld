MEMORY
{
    FLASH     (rx)  : org = 0x00100000, len = 32K
    RAM       (rwx) : org = 0x00200000, len = 16K
    STACK     (rw)  : org = 0x00300000, len = 8K
    INT_STACK (rw)  : org = 0x00400000, len = 8K
    UART      (rw)  : org = 0x40000000, len = 4
}
