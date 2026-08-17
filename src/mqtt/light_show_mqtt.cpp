#include "include/mqtt/light_show_mqtt.h"
#include "include/light_show_runner/light_show_runner.h"
#include "include/helpers/hw_infohelper.h"
#include "include/persistent_settings/persistent_settings.h"
#include <cstdio>
#include <algorithm>

namespace PicoLightShow {

LightShowMqtt::LightShowMqtt()
{
    mac = HWInfoHelper::GetMacAddress();
    baseTopic = "pico/" + mac + "/light/";
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
    Subscribe(baseTopic + "switch/set");
    Subscribe(baseTopic + "brightness/set");
    Subscribe(baseTopic + "rgb/set");
    Subscribe(baseTopic + "effect/set");

    // send start values
    PublishState();

    SendAutoDiscovery();
}

void LightShowMqtt::SendAutoDiscovery() 
{
    auto effects = LightShowRunner::GetEffectNames();
    std::string deviceName = "Pico Light Show " + mac.substr(mac.length() - 4);
    std::string deviceId = "pico_light_show_" + mac;
    std::string prefix = PersistentSettings::Settings.MqttDiscoveryTopic;

    std::string discoveryPayload = "{"
        "\"name\":\"" + deviceName + "\","
        "\"unique_id\":\"" + deviceId + "\","

        "\"cmd_t\":\"" + baseTopic + "switch/set\","
        "\"stat_t\":\"" + baseTopic + "switch/status\","

        // Brightness configuration
        "\"bri_cmd_t\":\"" + baseTopic + "brightness/set\","
        "\"bri_stat_t\":\"" + baseTopic + "brightness/status\","
        "\"bri_scl\":255,"

        // RGB configuration
        "\"rgb_cmd_t\":\"" + baseTopic + "rgb/set\","
        "\"rgb_stat_t\":\"" + baseTopic + "rgb/status\","
        "\"sup_clrm\":[\"rgb\"],"
        
        // Effects configurations
        "\"fx_cmd_t\":\"" + baseTopic + "effect/set\","
        "\"fx_stat_t\":\"" + baseTopic + "effect/status\","
        "\"fx_list\":[";

    for (size_t i = 0; i < effects.size(); ++i) 
    {
        discoveryPayload += "\"" + effects[i] + "\"";
        if (i < effects.size() - 1) 
            discoveryPayload += ",";
    }

    discoveryPayload += "],"
        "\"dev\":{"
            "\"identifiers\":[\"" + deviceId + "\"]," 
            "\"name\":\"" + deviceName + "\","
            "\"model\":\"Raspberry Pi Pico W\"," 
            "\"manufacturer\":\"Custom\""
        "}"
    "}";
    
    std::string discoveryTopic = prefix + "/light/" + deviceId + "/config";

    printf("[MQTT] Posilam discovery : %s\n", discoveryPayload.c_str());
    printf("[MQTT] Posilam discovery na topic: %s\n", discoveryTopic.c_str());

    Publish(discoveryTopic, discoveryPayload, true);
}

void LightShowMqtt::OnMqttMessage(const std::string& topic, const std::string& payload) 
{
    if (topic == baseTopic + "switch/set") {
        if (payload == "ON")
        {
            LightShowRunner::SwitchOn();
        }
        else
        {
            LightShowRunner::SwitchOff();
        }       

        Publish(baseTopic + "switch/status", LightShowRunner::GetSwitchOn() ? "ON" : "OFF", true);
    } 
    else if (topic == baseTopic + "brightness/set") {
        int val = std::stoi(payload);
        auto brightness = static_cast<uint8_t>(std::clamp(val, 0, 255));

        LightShowRunner::SetBrightness(brightness);
        
        Publish(baseTopic + "brightness/status", std::to_string(LightShowRunner::GetBrightness()), true);
    }
    else if (topic == baseTopic + "rgb/set") {
        uint8_t r, g, b;
        if (ParseRgbPayload(payload, r, g, b)) {           
            LightShowRunner::SetSolidColor(r, g, b);
            std::string rgbStr = std::to_string(r) + "," + std::to_string(g) + "," + std::to_string(b);
            
            Publish(baseTopic + "rgb/status", rgbStr, true);
        } else {
            printf("[MQTT] Error by parsing RGB payload (%s)!\n", payload.c_str());
        }
    }
    else if (topic == baseTopic + "effect/set") {
        auto effects = LightShowRunner::GetEffectNames();
        auto it = std::find(effects.begin(), effects.end(), payload);

        if (it != effects.end()) 
        {
            LightShowRunner::SetEffect(static_cast<u32_t>(std::distance(effects.begin(), it)));
        }

        Publish(baseTopic + "effect/status", LightShowRunner::GetEffectNames()[LightShowRunner::GetEffect()], true);
    }
}

void LightShowMqtt::PublishState() 
{
    Publish(baseTopic + "switch/status", LightShowRunner::GetSwitchOn() ? "ON" : "OFF", true);
    Publish(baseTopic + "brightness/status", std::to_string(LightShowRunner::GetBrightness()), true);
    
    Publish(baseTopic + "effect/status", LightShowRunner::GetEffectNames()[LightShowRunner::GetEffect()], true);

    Color solidColor = LightShowRunner::GetSolidColor();
    std::string rgbStr = std::to_string(solidColor.Red) + "," + std::to_string(solidColor.Green) + "," + std::to_string(solidColor.Blue);
    Publish(baseTopic + "rgb/status", rgbStr, true);
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