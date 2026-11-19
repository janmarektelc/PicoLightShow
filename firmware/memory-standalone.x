/* Diagnostic layout WITHOUT the bootloader (feature "standalone"):
 * the application starts right after BOOT2, OTA is disabled.
 * The STORAGE region is the same as in the normal layout, so settings are shared.
 */
MEMORY
{
  BOOT2   : ORIGIN = 0x10000000, LENGTH = 0x100
  FLASH   : ORIGIN = 0x10000100, LENGTH = 2032K - 0x100
  STORAGE : ORIGIN = 0x101FC000, LENGTH = 16K
  RAM     : ORIGIN = 0x20000000, LENGTH = 264K
}

/* OTA is disabled in this build; the symbols only satisfy the linker. */
__bootloader_state_start = 0;
__bootloader_state_end = 0;
__bootloader_dfu_start = 0;
__bootloader_dfu_end = 0;

__storage_start = ORIGIN(STORAGE) - ORIGIN(BOOT2);
__storage_end = ORIGIN(STORAGE) + LENGTH(STORAGE) - ORIGIN(BOOT2);

__app_start = ORIGIN(FLASH);
__app_end = ORIGIN(FLASH) + LENGTH(FLASH);
