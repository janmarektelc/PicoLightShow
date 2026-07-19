#include "include/wled_provider/wled_provider.h"
#include "include/light_show_runner/light_show_runner.h"
#include "include/helpers/hw_infohelper.h"
#include <cstdio>
#include <cstring>
#include <sstream>
#include "include/picojson.h"

namespace PicoLightShow
{   
    HttpResponse WLEDProvider::HandleSetJsonState(std::string data) {
        picojson::value v;
        std::istringstream stream(data);
        std::string err = picojson::parse(v, stream);

        if (!err.empty()) {
            HttpResponse res;
            res.statusCode = 400;
            res.contentType = "application/json";
            res.data = std::make_unique<RamSource>("{\"error\":\"Invalid JSON\"}");
            return res;
        }

        if (v.is<picojson::object>()) {
            const picojson::object& obj = v.get<picojson::object>();

            if (obj.count("ps") && obj.at("ps").is<double>()) {
                printf("set effect");
                uint32_t effect = static_cast<uint32_t>(obj.at("ps").get<double>()) - 1;
                LightShowRunner::SetEffect(effect);
            }

            if (obj.count("seg") && obj.at("seg").is<picojson::array>()) {
                const picojson::array& segments = obj.at("seg").get<picojson::array>();

                for (const auto& segVal : segments) {
                    if (!segVal.is<picojson::object>()) continue;
                    const picojson::object& seg = segVal.get<picojson::object>();

                    // Get segment id
                    int segId = 0;
                    if (seg.count("id") && seg.at("id").is<double>()) {
                        segId = static_cast<int>(seg.at("id").get<double>());
                    }

                    // Set segment speed
                    if (seg.count("sx") && seg.at("sx").is<double>()) {
                        LightShowRunner::SetFrameDelay(static_cast<uint8_t>(seg.at("sx").get<double>()));
                    }

                    // Set segment brightness
                    if (seg.count("ix") && seg.at("ix").is<double>()) {
                        LightShowRunner::SetBrightness(static_cast<uint8_t>(seg.at("ix").get<double>()));
                    }

                    // Set segment freeze
                    if (seg.count("frz") && seg.at("frz").is<bool>()) {
                        if (seg.at("frz").get<bool>()){
                            LightShowRunner::Stop();
                        }
                        else{
                            LightShowRunner::Start();
                        }
                    }
                }
            }
        }

        return HttpResponse::fromJsonString(WLEDProvider::HandleGetState());
    }

    std::string WLEDProvider::HandleGetState() {
        std::string led_count_str = std::to_string(LightShowRunner::GetLedCount()); 
        std::string brightness_str = std::to_string(LightShowRunner::GetBrightness());
        std::string effect_speed_str = std::to_string(LightShowRunner::GetFrameDelay());
        std::string current_effect_id_str = std::to_string(LightShowRunner::GetEffect() + 1);
        std::string freeze_str = LightShowRunner::GetIsRunning() ? "false" : "true";
        return R"({
            "on": true,
            "lor": 0,
            "bri": )" + brightness_str + R"(,
            "transition": 7,
            "ps": )" + current_effect_id_str + R"(,
            "pl": -1,
            "nl": {
            "on": false,
            "dur": 60,
            "fade": true,
            "tbri": 0
            },
            "udpn": {
                "send": false,
                "recv": false
            },
            "seg": [
            {
                "start": 0,
                "stop": )" + led_count_str + R"(,
                "len": )" + led_count_str + R"(,
                "frz": )" + freeze_str + R"(,
                "col": [[255, 160, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
                "fx": 0,
                "sx": )" + effect_speed_str + R"(,
                "ix": )" + brightness_str + R"(,
                "pal": 0,
                "sel": true,
                "rev": false,
                "cln": -1
            }
            ]
        })";
    }

    std::string WLEDProvider::HandleGetInfo() {
        std::string led_count_str = std::to_string(LightShowRunner::GetLedCount());

        return R"({
            "ver": "200.14.181",
            "vid": 1903252,
            "leds": {
                "count": )" + led_count_str + R"(,
                "rgbw": true,
                "pin": [2],
                "pwr": 0,
                "maxpwr": 65000,
                "maxseg": 1
            },
            "name": "Pico Light Show - WLED",
            "udpport": 21324,
            "live": false,
            "fxcount": 0,
            "palcount": 1,
            "arch": "rp2040",
            "core": "PicoSDK 2.0.0",
            "freeheap": )" + std::to_string(HWInfoHelper::GetFreeHeap()) + R"(,
            "uptime": )" + std::to_string(HWInfoHelper::GetUptime()) + R"(,
            "opt": 127,
            "brand": "JMSoftware",
            "product": "Pico Light Show",
            "btype": "src",
            "mac": ")" + HWInfoHelper::GetMacAddress() + R"(",
            "fs": {
                "u": 100,
                "t": 256,
                "pmt": 1718800000
            },
            "wifi": {
                "bssid": ")" + HWInfoHelper::GetBssid() + R"(",
                "rssi": ")" + std::to_string(HWInfoHelper::GetRssi()) + R"(",
                "signal": )" + std::to_string(HWInfoHelper::GetSignalPercentage()) + R"(,
                "channel": )" + std::to_string(HWInfoHelper::GetChannel()) + R"(
            }
        })";
    }

    std::string WLEDProvider::HandleGetJson() {
        std::string state_part = HandleGetState();
        std::string info_part = HandleGetInfo();        

        return "{\"state\":" + state_part + 
            ",\"info\":" + info_part + 
            ",\"palettes\":[\"Default\"]}";
    }

    std::string WLEDProvider::HandleGetPresets(){
        auto effectNames = LightShowRunner::GetEffectNames();

        std::ostringstream json;
        json << "{";

        for (size_t i = 0; i < effectNames.size(); ++i) {
            int presetId = static_cast<int>(i) + 1;
            if (i > 0) {
                json << ",";
            }
            json << "\"" << presetId << "\":{"
                << "\"n\":\"" << effectNames[i] << "\""
                << "}";
        }

        json << "}";
        return json.str();
    }
    
}// namespace PicoLightShow
