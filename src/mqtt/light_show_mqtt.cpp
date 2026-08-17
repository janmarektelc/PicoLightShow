#include "include/mqtt/light_show_mqtt.h"
#include "include/light_show_runner/light_show_runner.h"
#include "include/helpers/hw_infohelper.h"
#include <cstdio>
#include <algorithm>

namespace PicoLightShow {

LightShowMqtt::LightShowMqtt()
{
    //register callback for state change in LightShowRunner
    LightShowRunner::SetOnStateChangedCallback([this]() {
        if (IsConnected()) 
        {
            PublishState();
        }
    });
}

void LightShowMqtt::OnMqttConnected() 
{
    // subscriptions
    Subscribe("pico/light/switch");
    Subscribe("pico/light/brightness/set");
    Subscribe("pico/light/rgb/set");
    Subscribe("pico/light/effect/set");

    // send start values
    PublishState();

    SendAutoDiscovery();
}

void LightShowMqtt::SendAutoDiscovery() 
{
    const char* discoveryTopic = "homeassistant/light/pico_light_show/config";
    auto effects = LightShowRunner::GetEffectNames();

    std::string discoveryPayload = "{"
        "\"name\":\"Pico Light Show\","
        "\"unique_id\":\"pico_light_show_"+ HWInfoHelper::GetMacAddress() + "\","

        "\"cmd_t\":\"pico/light/switch\","
        "\"stat_t\":\"pico/light/status\","
        "\"pl_on\":\"ON\","
        "\"pl_off\":\"OFF\","

        // Brightness configuration
        "\"bri_cmd_t\":\"pico/light/brightness/set\","
        "\"bri_stat_t\":\"pico/light/brightness/status\","
        "\"bri_scl\":255,"

        // RGB configuration
        "\"rgb_cmd_t\":\"pico/light/rgb/set\","
        "\"rgb_stat_t\":\"pico/light/rgb/status\","
        "\"sup_clrm\":[\"rgb\"],"
        
        // Effects configurations
        "\"fx_cmd_t\":\"pico/light/effect/set\","
        "\"fx_stat_t\":\"pico/light/effect\","
        "\"fx_list\":[";
    for (size_t i = 0; i < effects.size(); ++i) 
    {
        discoveryPayload += "\"" + effects[i] + "\"";
        if (i < effects.size() - 1) 
            discoveryPayload += ",";
    }
    discoveryPayload += "]}";

    Publish(discoveryTopic, discoveryPayload, true);
}

void LightShowMqtt::OnMqttMessage(const std::string& topic, const std::string& payload) 
{
    if (topic == "pico/light/switch") {
        if (payload == "ON")
        {
            LightShowRunner::SwitchOn();
        }
        else
        {
            LightShowRunner::SwitchOff();
        }       

        Publish("pico/light/status", LightShowRunner::GetSwitchOn() ? "ON" : "OFF", true);
    } 
    else if (topic == "pico/light/brightness/set") {
        int val = std::stoi(payload);
        auto brightness = static_cast<uint8_t>(std::clamp(val, 0, 255));

        LightShowRunner::SetBrightness(brightness);
        
        Publish("pico/light/brightness/status", std::to_string(LightShowRunner::GetBrightness()), true);
    }
    else if (topic == "pico/light/rgb/set") {
        uint8_t r, g, b;
        if (ParseRgbPayload(payload, r, g, b)) {           
            LightShowRunner::SetSolidColor(r, g, b);
            std::string rgbStr = std::to_string(r) + "," + std::to_string(g) + "," + std::to_string(b);
            Publish("pico/light/rgb/status", rgbStr, true);
        } else {
            printf("[MQTT] Error by parsing RGB payload (%s)!\n", payload.c_str());
        }
    }
    else if (topic == "pico/light/effect/set") {
        auto effects = LightShowRunner::GetEffectNames();
        auto it = std::find(effects.begin(), effects.end(), payload);

        if (it != effects.end()) 
        {
            LightShowRunner::SetEffect(static_cast<u32_t>(std::distance(effects.begin(), it)));
        }

        Publish("pico/light/effect", LightShowRunner::GetEffectNames()[LightShowRunner::GetEffect()], true);
    }
}

void LightShowMqtt::PublishState() 
{
    Publish("pico/light/status", LightShowRunner::GetSwitchOn() ? "ON" : "OFF", true);
    Publish("pico/light/brightness/status", std::to_string(LightShowRunner::GetBrightness()), true);
    Publish("pico/light/effect", LightShowRunner::GetEffectNames()[LightShowRunner::GetEffect()], true);

    Color solidColor = LightShowRunner::GetSolidColor();
    std::string rgbStr = std::to_string(solidColor.Red) + "," + std::to_string(solidColor.Green) + "," + std::to_string(solidColor.Blue);
    Publish("pico/light/rgb/status", rgbStr, true);
}

bool LightShowMqtt::ParseRgbPayload(const std::string& payload, uint8_t& r, uint8_t& g, uint8_t& b) 
{
    int rVal, gVal, bVal;

    if (sscanf(payload.c_str(), "%d,%d,%d", &rVal, &gVal, &bVal) == 3) {
        r = static_cast<uint8_t>(std::clamp(rVal, 0, 255));
        g = static_cast<uint8_t>(std::clamp(gVal, 0, 255));
        b = static_cast<uint8_t>(std::clamp(bVal, 0, 255));
        return true;
    }

    return false;
}

} // namespace PicoLightShow