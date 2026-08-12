#include "include/mqtt/light_show_mqtt.h"
#include "include/light_show_runner/light_show_runner.h"
#include <cstdio>
#include <algorithm>

namespace PicoLightShow {

void LightShowMqtt::OnMqttConnected() {
    SendAutoDiscovery();

    // subscriptions
    Subscribe("pico/light/switch");
    Subscribe("pico/light/brightness/set");
    Subscribe("pico/light/effect/set");

    // send start values
    Publish("pico/light/status", LightShowRunner::GetIsRunning() ? "ON" : "OFF", true);
    Publish("pico/light/brightness/status", std::to_string(LightShowRunner::GetBrightness()), true);
    Publish("pico/light/effect", LightShowRunner::GetEffectNames()[LightShowRunner::GetEffect()], true);
}

void LightShowMqtt::SendAutoDiscovery() {
    const char* discoveryTopic = "homeassistant/light/pico_light_show/config";
    auto effects = LightShowRunner::GetEffectNames();

    std::string discoveryPayload = "{"
        "\"name\":\"Pico Light Show\","
        "\"unique_id\":\"pico_light_show_02\","
        "\"cmd_t\":\"pico/light/switch\","
        "\"stat_t\":\"pico/light/status\","

        // Brightness configuration
        "\"bri_cmd_t\":\"pico/light/brightness/set\","
        "\"bri_stat_t\":\"pico/light/brightness/status\","
        "\"bri_scl\":255,"
        
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

void LightShowMqtt::OnMqttMessage(const std::string& topic, const std::string& payload) {
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

} // namespace PicoLightShow