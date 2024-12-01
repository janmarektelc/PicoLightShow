#include "hardware/flash.h"
#include "hardware/sync.h"
#include "string.h"
#include <vector>
#include <string>

#include "include/constants.h"
#include "include/helpers/string_helper.h"
#include "include/persistent_settings/persistent_settings.h"

#define FLASH_DATA_IDENTIFIER 0x7E
#define FLASH_TARGET_OFFSET (PICO_FLASH_SIZE_BYTES - FLASH_SECTOR_SIZE)

namespace PicoLightShow
{
    uint32_t convertIpToUint32(const uint8_t ip0, const uint8_t ip1, const uint8_t ip2, const uint8_t ip3) {
    return (static_cast<uint32_t>(ip3) << 24) |
           (static_cast<uint32_t>(ip2) << 16) |
           (static_cast<uint32_t>(ip1) << 8)  |
           static_cast<uint32_t>(ip0);
    };

    settings PersistentSettings::GetDefaultSettigns()
    {
        return {
            .WifiName = DEFAULT_WIFI_SSID,
            .WifiPassword = DEFAULT_WIFI_PASSWORD,
            .WifiMode = AP,
            .IsDhcp = true,
            .DeviceAddress = convertIpToUint32(0,0,0,0),
            .NetMask = convertIpToUint32(0,0,0,0),
            .GatewayAddress = convertIpToUint32(0,0,0,0),
            .IsRunning = true,
            .Delay = 20,
            .EffectIndex = 0,
            .LedCount = 20,
            .Brightness = 50,
            .CurrentEffectConfiguration = "",
        }; 
    }

    settings PersistentSettings::Settings = PersistentSettings::GetDefaultSettigns();

    void PersistentSettings::Load()
    {
        Load(&Settings, sizeof(Settings));
    }

    void PersistentSettings::Save()
    {
        Save(&Settings, sizeof(Settings));
    }

    void PersistentSettings::SetToDefault()
    {
        Settings = GetDefaultSettigns();
    }

    void PersistentSettings::SetByConfigString(std::string cfg)
    {
        //expected data format
        //~wifiMode(ap=0/client=1)~ssid~passwd~isdhcp(0/1)~ipAddress~ipMask~gwIp~
        std::vector<std::string> segments = StringHelper::Split(cfg, '~');

        Settings.WifiMode = segments[1] == "0" ? AP : CLIENT;
        strcpy(Settings.WifiName, segments[2].c_str());
        if (segments[3] != "*^NotChanged^*")
        {
            if (segments[3] == "*^Empty^*")
                strcpy(Settings.WifiPassword, "");
            else
                strcpy(Settings.WifiPassword, segments[3].c_str());
        }
        Settings.IsDhcp = segments[4] == "1";

        std::vector<std::string> address = StringHelper::Split(segments[5], '.');
        Settings.DeviceAddress = convertIpToUint32(atoi(address[0].c_str()), atoi(address[1].c_str()), atoi(address[2].c_str()), atoi(address[3].c_str()));
        address = StringHelper::Split(segments[6], '.');
        Settings.NetMask = convertIpToUint32(atoi(address[0].c_str()), atoi(address[1].c_str()), atoi(address[2].c_str()), atoi(address[3].c_str()));
        address = StringHelper::Split(segments[7], '.');
        Settings.GatewayAddress= convertIpToUint32(atoi(address[0].c_str()), atoi(address[1].c_str()), atoi(address[2].c_str()), atoi(address[3].c_str()));
    }

    int PersistentSettings::GetMemoryPage()
    {
        for (int page = 0; page < FLASH_SECTOR_SIZE / FLASH_PAGE_SIZE; page++)
        {
            const uint32_t *ptr = (const uint32_t *)(XIP_BASE + FLASH_TARGET_OFFSET + FLASH_PAGE_SIZE * page);
            if (ptr[0] == 0xffffffff)
                return page;
        }
        return -1;
    }

    void PersistentSettings::Load(void *buffer, const uint32_t size)
    {
        int page = GetMemoryPage();
        if (page == -1)
            page = FLASH_SECTOR_SIZE / FLASH_PAGE_SIZE;
        page--;
        const uint8_t *flash_data = (const uint8_t *)(XIP_BASE + FLASH_TARGET_OFFSET + FLASH_PAGE_SIZE * page);
        if (flash_data[0] != FLASH_DATA_IDENTIFIER)
            return;

        memcpy(buffer, flash_data + 1, size);
    }

    void PersistentSettings::Save(void *buffer, const uint32_t size)
    {
        int page = GetMemoryPage();
        if (page == -1)
        {
            flash_range_erase(FLASH_TARGET_OFFSET, FLASH_SECTOR_SIZE);
            page = 0;
        }

        uint8_t flash_data[FLASH_PAGE_SIZE];
        memset(flash_data, 0xff, FLASH_PAGE_SIZE);
        flash_data[0] = FLASH_DATA_IDENTIFIER;
        memcpy(flash_data + 1, buffer, size);
        const uint32_t ints = save_and_disable_interrupts();
        flash_range_program(FLASH_TARGET_OFFSET + FLASH_PAGE_SIZE * page, flash_data, FLASH_PAGE_SIZE);
        restore_interrupts(ints);
    }
}
