#pragma once

#include "lwip/tcp.h"
#include "lwip/ip_addr.h"
#include <string>
#include <vector>

namespace PicoLightShow {

enum class MqttState {
    DISCONNECTED,
    CONNECTING,
    CONNECTED
};

class MqttClient {
public:
    MqttClient();
    virtual ~MqttClient();

    void Poll();

    void Connect(const char* brokerIp, uint16_t port, const char* clientId, const char* user = nullptr, const char* pass = nullptr);
    void Disconnect();

    bool Publish(const std::string& topic, const std::string& payload, bool retain = false);
    bool Subscribe(const std::string& topic);

    bool IsConnected() const { return m_state == MqttState::CONNECTED; }

protected:
    virtual void OnMqttConnected() {}
    virtual void OnMqttDisconnected() {}
    virtual void OnMqttMessage(const std::string& topic, const std::string& payload) {}
    void Connect();

private:
    tcp_pcb* m_pcb = nullptr;
    MqttState m_state = MqttState::DISCONNECTED;
    uint32_t m_lastSendTime = 0;

    std::string m_clientId;
    std::string m_username;
    std::string m_password;
    std::string m_brokerIpStr;
    uint16_t m_port;

    static void EncodeLength(size_t length, std::vector<uint8_t>& buffer);
    bool SendRawData(const uint8_t* data, size_t len);
    void SendPing();

    static err_t OnTcpConnected(void* arg, tcp_pcb* tpcb, err_t err);
    static err_t OnTcpReceive(void* arg, tcp_pcb* tpcb, pbuf* p, err_t err);
    static void OnTcpError(void* arg, err_t err);
    
    void ProcessIncomingPacket(const uint8_t* data, size_t len);
};

} // namespace PicoLightShow
