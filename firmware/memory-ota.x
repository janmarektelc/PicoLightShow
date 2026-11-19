/* Flash layout of the Pico W (2 MiB) shared by the bootloader and the application.
 *
 *  0x10000000  BOOT2 + bootloader            24 KiB   (bootloader crate)
 *  0x10006000  BOOTLOADER_STATE               4 KiB   (OTA state / rollback flags)
 *  0x10007000  ACTIVE  (this application)  1000 KiB
 *  0x10101000  DFU     (OTA download slot) 1004 KiB   (must be ACTIVE + 4 KiB)
 *  0x101FC000  STORAGE (settings)            16 KiB   (sequential-storage map)
 *
 * Keep in sync with bootloader/memory.x and memory-standalone.x and xtask/src/main.rs.
 */
MEMORY
{
  BOOT2            : ORIGIN = 0x10000000, LENGTH = 0x100
  BOOTLOADER_STATE : ORIGIN = 0x10006000, LENGTH = 4K
  FLASH            : ORIGIN = 0x10007000, LENGTH = 1000K
  DFU              : ORIGIN = 0x10101000, LENGTH = 1004K
  STORAGE          : ORIGIN = 0x101FC000, LENGTH = 16K
  RAM              : ORIGIN = 0x20000000, LENGTH = 264K
}

__bootloader_state_start = ORIGIN(BOOTLOADER_STATE) - ORIGIN(BOOT2);
__bootloader_state_end = ORIGIN(BOOTLOADER_STATE) + LENGTH(BOOTLOADER_STATE) - ORIGIN(BOOT2);

__bootloader_dfu_start = ORIGIN(DFU) - ORIGIN(BOOT2);
__bootloader_dfu_end = ORIGIN(DFU) + LENGTH(DFU) - ORIGIN(BOOT2);

__storage_start = ORIGIN(STORAGE) - ORIGIN(BOOT2);
__storage_end = ORIGIN(STORAGE) + LENGTH(STORAGE) - ORIGIN(BOOT2);

__app_start = ORIGIN(FLASH);
__app_end = ORIGIN(FLASH) + LENGTH(FLASH);
