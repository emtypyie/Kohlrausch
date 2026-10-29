/* Kohlrausch RP2040: Reserve the final 16 KB of flash for dual-slot config storage.
 *
 * Slot A is located at PICO_FLASH_SIZE_BYTES - 16384 (0x101FC000 on 2MB flash)
 * Slot B is located at PICO_FLASH_SIZE_BYTES - 8192  (0x101FE000 on 2MB flash)
 *
 * Restricting the FLASH region to 2032 KB (2MB - 16KB) prevents the linker from
 * ever placing application code, rodata, or vectors into these persistence sectors.
 */
FLASH(rx) : ORIGIN = 0x10000000, LENGTH = (2 * 1024 * 1024) - 16k
