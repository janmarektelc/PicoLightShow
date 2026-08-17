#pragma once

#include "mqtt_client.h"

namespace PicoLightShow {

class LightShowMqtt : public MqttClient {
public:
    LightShowMqtt();

protected:
    // Implementation of metods from MqttClient
    void OnMqttConnected() override;
    void OnMqttMessage(const std::string& topic, const std::string& payload) override;

private:
    void SendAutoDiscovery();
    void PublishState();
    bool ParseRgbPayload(const std::string& payload, uint8_t& r, uint8_t& g, uint8_t& b);
};

} // namespace PicoLightShow