#include "pico/stdlib.h"
#include "pico/cyw43_arch.h"
#include "pico/multicore.h"

extern "C" {
#include "dhcpserver/dhcpserver.h"
#include "dnsserver/dnsserver.h"
}

#include "include/www_server/light_show_web.h"
#include "include/light_show_runner/light_show_runner.h"
#include "include/persistent_settings/persistent_settings.h"
#include "include/ddp/ddp.h"
#include "include/mqtt/mqtt_client.h"
#include "include/mqtt/light_show_mqtt.h"
#include "include/helpers/string_helper.h"

void startAp(const char* ssid, const char* password) {
    cyw43_arch_enable_ap_mode(ssid, password, CYW43_AUTH_WPA2_AES_PSK);

    ip4_addr_t gw, mask;
    IP4_ADDR(&gw, 192, 168, 4, 1);
    IP4_ADDR(&mask, 255, 255, 255, 0);

    static dhcp_server_t dhcp_server;
    dhcp_server_init(&dhcp_server, &gw, &mask);

    static dns_server_t dns_server;
    dns_server_init(&dns_server, &gw);
}

void core1_entry()
{
    PicoLightShow::LightShowRunner::Init();

    while (true) {
        PicoLightShow::LightShowRunner::Pool();
    }

}

int main()
{
    stdio_init_all();

    if (cyw43_arch_init())
        return 1;

    PicoLightShow::PersistentSettings::Load();

    multicore_launch_core1(core1_entry);

    bool isConnected = false; //true if pico is connected to wifi network as a client

    if (PicoLightShow::PersistentSettings::Settings.WifiMode == PicoLightShow::wifiModeEnum::CLIENT)
    {
        // Enable wifi station
        cyw43_arch_enable_sta_mode();  

        if (cyw43_arch_wifi_connect_timeout_ms(PicoLightShow::PersistentSettings::Settings.WifiName, PicoLightShow::PersistentSettings::Settings.WifiPassword, CYW43_AUTH_WPA2_AES_PSK, 30000)) {
            isConnected = false;
        } else {
            isConnected = true;
        }

        if (isConnected && !PicoLightShow::PersistentSettings::Settings.IsDhcp)        
        {
            dhcp_release_and_stop(cyw43_state.netif);
            ip4_addr_t ip, gw, mask;
            ip.addr=PicoLightShow::PersistentSettings::Settings.DeviceAddress;
            mask.addr=PicoLightShow::PersistentSettings::Settings.NetMask;
            gw.addr=PicoLightShow::PersistentSettings::Settings.GatewayAddress;
            netif_set_addr(cyw43_state.netif, &ip, &mask, &gw);
            netif_set_up(cyw43_state.netif);
        }

        if (!isConnected)
        {
            // not possible to connect to configured wifi, start default AP mode
            startAp(DEFAULT_WIFI_SSID, DEFAULT_WIFI_PASSWORD);
        }
    }

    if (PicoLightShow::PersistentSettings::Settings.WifiMode == PicoLightShow::wifiModeEnum::AP)
    {
        // Enable wifi access point
        startAp(PicoLightShow::PersistentSettings::Settings.WifiName, PicoLightShow::PersistentSettings::Settings.WifiPassword);
    }

    PicoLightShow::LightShowWeb webServer;
    webServer.start(80);

    PicoLightShow::LightShowMqtt mqtt;
    if (isConnected) { // if is connected to wifi network as a client, connect to mqtt broker and initialize DDP
        PicoLightShow::DDP::Init(PicoLightShow::PersistentSettings::Settings.LedCount);

        if (PicoLightShow::PersistentSettings::Settings.IsMqttEnabled)
        {
            mqtt.Connect(PicoLightShow::StringHelper::ConvertUint32ToIpString(PicoLightShow::PersistentSettings::Settings.MqttServerAddress).c_str(), 
                PicoLightShow::PersistentSettings::Settings.MqttServerPort, 
                "PicoLightShowClient", 
                PicoLightShow::PersistentSettings::Settings.MqttUsername, 
                PicoLightShow::PersistentSettings::Settings.MqttPassword);
        }
    }

    while (true) {
        if (isConnected)
        {
            PicoLightShow::DDP::Poll();
            if (PicoLightShow::PersistentSettings::Settings.IsMqttEnabled)
            {
                mqtt.Poll();
            }
        }
        cyw43_arch_poll();
    }

    cyw43_arch_deinit();
}
