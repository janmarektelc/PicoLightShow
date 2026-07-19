
#include "include/helpers/hw_infohelper.h"
#include "pico/cyw43_arch.h"
#include "pico/time.h"

namespace PicoLightShow
{
    std::string HWInfoHelper::LoadMacAddress() {
        uint8_t mac[6];
        cyw43_wifi_get_mac(&cyw43_state, CYW43_ITF_STA, mac);

        char temp_str[13];
        std::snprintf(temp_str, sizeof(temp_str), "%02x%02x%02x%02x%02x%02x", mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]);
                    
        return std::string(temp_str);
    }

    uint32_t HWInfoHelper::GetUptime() {
        return static_cast<uint32_t>(time_us_64() / 1000000ULL);
    }

    const std::string& HWInfoHelper::GetMacAddress() {
        static const std::string mac_address = LoadMacAddress();
        return mac_address;
    }

    extern "C" {
        extern char __StackLimit[];
        extern char __heap_start[];
    }

    uint32_t HWInfoHelper::GetFreeHeap() {
        char *heap_end;
        heap_end = (char*)__builtin_frame_address(0);
        return heap_end - __heap_start;
    }

    const std::string& HWInfoHelper::GetBssid() {
        static const std::string bssid_str = []() {
            uint8_t bssid[6] = {0};
            cyw43_wifi_get_bssid(&cyw43_state, bssid);

            char temp[13];
            std::snprintf(temp, sizeof(temp), "%02x%02x%02x%02x%02x%02x", 
                        bssid[0], bssid[1], bssid[2], bssid[3], bssid[4], bssid[5]);
            return std::string(temp);
        }();
        return bssid_str;
    }

    int32_t HWInfoHelper::GetRssi() {
        int32_t rssi = 0;
        cyw43_wifi_get_rssi(&cyw43_state, &rssi);
        return rssi;
    }

    int HWInfoHelper::GetSignalPercentage() {
        int32_t rssi = GetRssi();
        if (rssi >= -50) return 100;
        if (rssi <= -100) return 0;
        return 2 * (rssi + 100);
    }

    int HWInfoHelper::GetChannel() {
        int32_t channel = 0;
        cyw43_ioctl(&cyw43_state, 0x3a, sizeof(channel), (uint8_t*)&channel, CYW43_ITF_STA);
        return static_cast<int>(channel);
    }
}