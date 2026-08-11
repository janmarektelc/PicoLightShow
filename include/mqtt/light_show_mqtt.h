#pragma once

#include "mqtt_client.h"

namespace PicoLightShow {

class LightShowMqtt : public MqttClient {
public:
    LightShowMqtt() = default;

protected:
    // Implementation of metods from MqttClient
    void OnMqttConnected() override;
    void OnMqttMessage(const std::string& topic, const std::string& payload) override;

private:
    void SendAutoDiscovery();
};

} // namespace PicoLightShow