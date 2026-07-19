#pragma once
#include <string>
#include <cstdint>

namespace PicoLightShow
{
    class HWInfoHelper {
    public:
        HWInfoHelper() = delete;
        ~HWInfoHelper() = delete;

        static const std::string& GetMacAddress();
        static uint32_t GetUptime();
        static uint32_t GetFreeHeap();

        // Wi-Fi info (BSSID, RSSI, Signal %, Channel)
        static const std::string& GetBssid();
        static int32_t GetRssi();
        static int GetSignalPercentage();
        static int GetChannel();
    private:
        static std::string LoadMacAddress();
    };

} // namespace PicoLightShow