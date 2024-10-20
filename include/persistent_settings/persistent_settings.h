#ifndef PICO_LIGHT_SHOW_PERSISTENT_SETTINGS_H_
#define PICO_LIGHT_SHOW_PERSISTENT_SETTINGS_H_

#include <stdio.h>
#include <stdint.h>
#include <string>

namespace PicoLightShow
{
    enum wifiModeEnum
    {
        AP,
        CLIENT
    };

    struct settings
    {
        char WifiName[32 + 1];
        char WifiPassword[64 + 1];
        wifiModeEnum WifiMode;
        bool IsDhcp;
        uint32_t DeviceAddress;
        uint32_t NetMask;
        uint32_t GatewayAddress;
        bool IsRunning;
        uint32_t Delay;
        uint8_t EffectIndex;
        uint32_t LedCount;
        uint8_t Brightness;
    };

    class PersistentSettings
    {
    public:
        PersistentSettings() = delete;
        static void Save();
        static void Load();
        static void SetToDefault();
        static void SetByConfigString(std::string cfg);
        static settings Settings;
    
    private:
        static int GetMemoryPage();
        static void Load(void *buffer, const uint32_t size);
        static void Save(void *buffer, const uint32_t size);
        static settings GetDefaultSettigns();
    };

} // namespace PicoLightShow

#endif // PICO_LIGHT_SHOW_PERSISTENT_SETTINGS_H_