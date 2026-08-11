#include "include/mqtt/mqtt_client.h"
#include <cstdio>
#include <cstring>
#include <algorithm>
#include <pico/time.h>

namespace PicoLightShow {

MqttClient::MqttClient() = default;

MqttClient::~MqttClient() {
    Disconnect();
}

void MqttClient::Poll() {
    uint32_t now = to_ms_since_boot(get_absolute_time());

    if (m_state == MqttState::CONNECTED) {
        //if last 30s no data sent, send ping
        if (now - m_lastSendTime >= 30000) {
            SendPing();
        }
    }
}

void MqttClient::EncodeLength(size_t length, std::vector<uint8_t>& buffer) {
    do {
        uint8_t encodedByte = length % 128;
        length /= 128;
        if (length > 0) {
            encodedByte |= 128;
        }
        buffer.push_back(encodedByte);
    } while (length > 0);
}

void MqttClient::Connect(const char* brokerIpStr, uint16_t port, const char* clientId, const char* user, const char* pass) {
    if (m_state != MqttState::DISCONNECTED) return;

    m_clientId = clientId ? clientId : "PicoClient";
    m_username = user ? user : "";
    m_password = pass ? pass : "";

    ip_addr_t brokerIp;
    if (!ipaddr_aton(brokerIpStr, &brokerIp)) {
        printf("[MQTT-BASE] Error: Wrong mqtt broker address!\n");
        return;
    }

    m_pcb = tcp_new();
    if (!m_pcb) {
        printf("[MQTT-BASE] Error: can't allocate PCB!\n");
        return;
    }

    tcp_arg(m_pcb, this);
    tcp_err(m_pcb, OnTcpError);
    tcp_recv(m_pcb, OnTcpReceive);

    m_state = MqttState::CONNECTING;

    tcp_connect(m_pcb, &brokerIp, port, OnTcpConnected);
}

err_t MqttClient::OnTcpConnected(void* arg, tcp_pcb* tpcb, err_t err) {
    auto* self = static_cast<MqttClient*>(arg);
    if (err != ERR_OK) {
        self->m_state = MqttState::DISCONNECTED;
        self->OnMqttDisconnected();
        return err;
    }

    // MQTT CONNECT packet
    std::vector<uint8_t> varHeader = {
        0x00, 0x04, 'M', 'Q', 'T', 'T', // Protocol Name
        0x04,                           // Protocol Level v3.1.1
        0x02,                           // Clean Session
        0x00, 0x3C                      // Keep Alive 60s
    };

    uint8_t flags = 0x02;
    if (!self->m_username.empty()) flags |= 0x80;
    if (!self->m_password.empty()) flags |= 0x40;
    varHeader[7] = flags;

    std::vector<uint8_t> payload;
    // Client ID
    payload.push_back(self->m_clientId.length() >> 8);
    payload.push_back(self->m_clientId.length() & 0xFF);
    payload.insert(payload.end(), self->m_clientId.begin(), self->m_clientId.end());

    // User/Pass
    if (!self->m_username.empty()) {
        payload.push_back(self->m_username.length() >> 8);
        payload.push_back(self->m_username.length() & 0xFF);
        payload.insert(payload.end(), self->m_username.begin(), self->m_username.end());
    }
    if (!self->m_password.empty()) {
        payload.push_back(self->m_password.length() >> 8);
        payload.push_back(self->m_password.length() & 0xFF);
        payload.insert(payload.end(), self->m_password.begin(), self->m_password.end());
    }

    std::vector<uint8_t> packet;
    packet.push_back(0x10); // CONNECT Control Byte
    EncodeLength(varHeader.size() + payload.size(), packet);
    packet.insert(packet.end(), varHeader.begin(), varHeader.end());
    packet.insert(packet.end(), payload.begin(), payload.end());

    self->SendRawData(packet.data(), packet.size());
    return ERR_OK;
}

bool MqttClient::Publish(const std::string& topic, const std::string& payload, bool retain) {
    if (m_state != MqttState::CONNECTED || !m_pcb) return false;

    std::vector<uint8_t> varHeader;
    varHeader.push_back(topic.length() >> 8);
    varHeader.push_back(topic.length() & 0xFF);
    varHeader.insert(varHeader.end(), topic.begin(), topic.end());

    uint8_t headerByte = 0x30; // PUBLISH (QoS 0)
    if (retain) headerByte |= 0x01;

    std::vector<uint8_t> packet;
    packet.push_back(headerByte);
    EncodeLength(varHeader.size() + payload.length(), packet);
    packet.insert(packet.end(), varHeader.begin(), varHeader.end());

    // Send MQTT header
    if (!SendRawData(packet.data(), packet.size())) return false;
    
    // Send payload
    return SendRawData(reinterpret_cast<const uint8_t*>(payload.data()), payload.length());
}

bool MqttClient::Subscribe(const std::string& topic) {
    if (m_state != MqttState::CONNECTED || !m_pcb) return false;

    static uint16_t packetId = 1;
    std::vector<uint8_t> body;
    body.push_back(packetId >> 8);
    body.push_back(packetId & 0xFF);
    packetId++;

    body.push_back(topic.length() >> 8);
    body.push_back(topic.length() & 0xFF);
    body.insert(body.end(), topic.begin(), topic.end());
    body.push_back(0x00); // QoS 0

    std::vector<uint8_t> packet;
    packet.push_back(0x82); // SUBSCRIBE
    EncodeLength(body.size(), packet);
    packet.insert(packet.end(), body.begin(), body.end());

    return SendRawData(packet.data(), packet.size());
}

bool MqttClient::SendRawData(const uint8_t* data, size_t len) {
    if (!m_pcb) return false;

    size_t offset = 0;
    while (offset < len) {
        size_t freeSpace = tcp_sndbuf(m_pcb);
        if (freeSpace == 0) {
            tcp_output(m_pcb);
            freeSpace = tcp_sndbuf(m_pcb);
            if (freeSpace == 0) return false;
        }

        size_t toWrite = std::min(len - offset, freeSpace);
        err_t err = tcp_write(m_pcb, data + offset, toWrite, TCP_WRITE_FLAG_COPY);
        if (err != ERR_OK) {
            printf("[MQTT-BASE] tcp_write failed: %d\n", err);
            return false;
        }

        offset += toWrite;
    }
    tcp_output(m_pcb);
    m_lastSendTime = to_ms_since_boot(get_absolute_time());
    return true;
}

void MqttClient::SendPing() {
    uint8_t pingPacket[] = { 0xC0, 0x00 }; // MQTT PINGREQ header
    SendRawData(pingPacket, sizeof(pingPacket));
}

err_t MqttClient::OnTcpReceive(void* arg, tcp_pcb* tpcb, pbuf* p, err_t err) {
    auto* self = static_cast<MqttClient*>(arg);
    if (!p) {
        printf("[MQTT-BASE] connection closed by server.\n");
        self->Disconnect();
        return ERR_OK;
    }

    self->ProcessIncomingPacket(static_cast<const uint8_t*>(p->payload), p->tot_len);

    tcp_recved(tpcb, p->tot_len);
    pbuf_free(p);
    return ERR_OK;
}

void MqttClient::ProcessIncomingPacket(const uint8_t* data, size_t len) {
    if (len < 2) return;

    uint8_t msgType = data[0] & 0xF0;

    // 0x20 = CONNACK
    if (msgType == 0x20) {
        if (len >= 4 && data[3] == 0x00) {
            m_state = MqttState::CONNECTED;
            OnMqttConnected(); // hook for inherited class
        }
    }
    // 0x30 = PUBLISH
    else if (msgType == 0x30) {
        size_t idx = 1;
        // Skip remainng length byte(s)
        while (idx < len && (data[idx] & 0x80)) idx++;
        idx++;

        if (idx + 2 <= len) {
            uint16_t topicLen = (data[idx] << 8) | data[idx + 1];
            idx += 2;

            if (idx + topicLen <= len) {
                std::string topic(reinterpret_cast<const char*>(data + idx), topicLen);
                idx += topicLen;

                std::string payload(reinterpret_cast<const char*>(data + idx), len - idx);
                
                // call inherited virtual method
                OnMqttMessage(topic, payload);
            }
        }
    }
}

void MqttClient::OnTcpError(void* arg, err_t err) {
    auto* self = static_cast<MqttClient*>(arg);
    self->m_state = MqttState::DISCONNECTED;
    self->m_pcb = nullptr;
    self->OnMqttDisconnected();
}

void MqttClient::Disconnect() {
    if (m_pcb) {
        tcp_arg(m_pcb, nullptr);
        tcp_close(m_pcb);
        m_pcb = nullptr;
    }
    m_state = MqttState::DISCONNECTED;
}

} // namespace PicoLightShow
