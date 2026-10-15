MEMORY
{
    FLASH    (rx)  : org = 0x00100000, len = 0x4000
    RAM      (rwx) : org = 0x00200000, len = 0x4000
    STACK    (rw)  : org = 0x00300000, len = 0x2000
    UART     (rw)  : org = 0x40000000, len = 0x4
}
