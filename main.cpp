//#define CYW43_HOST_NAME "PicoLightShow"

#include "pico/stdlib.h"
#include "pico/cyw43_arch.h"
#include "pico/multicore.h"

extern "C" {
#include "dhcpserver/dhcpserver.h"
#include "dnsserver/dnsserver.h"
}

#include "include/www_server/www_server.h"
#include "include/light_show_runner/light_show_runner.h"
#include "include/persistent_settings/persistent_settings.h"

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

    bool isConnected = false;

    if (PicoLightShow::PersistentSettings::Settings.WifiMode == PicoLightShow::wifiModeEnum::CLIENT)
    {
        // Enable wifi station
        cyw43_arch_enable_sta_mode();  

        if (!PicoLightShow::PersistentSettings::Settings.IsDhcp)
        {
#if NO_SYS == 1
		cyw43_arch_lwip_begin();
#endif
        dhcp_release_and_stop(cyw43_state.netif);
        ip4_addr_t ip, gw, mask;
        ip.addr=PicoLightShow::PersistentSettings::Settings.DeviceAddress;
        mask.addr=PicoLightShow::PersistentSettings::Settings.NetMask;
        gw.addr=PicoLightShow::PersistentSettings::Settings.GatewayAddress;
        netif_set_addr(cyw43_state.netif, &ip, &mask, &gw);

#if NO_SYS == 1
		cyw43_arch_lwip_end();
#endif
        //netif_set_hostname(cyw43_state.netif, "PicoLightShow");
        }

        if (cyw43_arch_wifi_connect_timeout_ms(PicoLightShow::PersistentSettings::Settings.WifiName, PicoLightShow::PersistentSettings::Settings.WifiPassword, CYW43_AUTH_WPA2_AES_PSK, 30000)) {
            isConnected = false;
        } else {
            isConnected = true;
        }

        if (!isConnected)
        {
            // not possible to connect to configured wifi, start default AP mode
            cyw43_arch_enable_ap_mode(DEFAULT_WIFI_SSID, DEFAULT_WIFI_PASSWORD, CYW43_AUTH_WPA2_AES_PSK);

            ip4_addr_t gw, mask;
            IP4_ADDR(&gw, 192, 168, 4, 1);
            IP4_ADDR(&mask, 255, 255, 255, 0);

            dhcp_server_t dhcp_server;
            dhcp_server_init(&dhcp_server, &gw, &mask);

            dns_server_t dns_server;
            dns_server_init(&dns_server, &gw);
            }
    }

    if (PicoLightShow::PersistentSettings::Settings.WifiMode == PicoLightShow::wifiModeEnum::AP)
    {
        //netif_set_hostname(cyw43_state.netif, "PicoLightShow");
        // Enable wifi access point
        cyw43_arch_enable_ap_mode(PicoLightShow::PersistentSettings::Settings.WifiName, PicoLightShow::PersistentSettings::Settings.WifiPassword, CYW43_AUTH_WPA2_AES_PSK);

        ip4_addr_t gw, mask;
        IP4_ADDR(&gw, 192, 168, 4, 1);
        IP4_ADDR(&mask, 255, 255, 255, 0);

        dhcp_server_t dhcp_server;
        dhcp_server_init(&dhcp_server, &gw, &mask);

        dns_server_t dns_server;
        dns_server_init(&dns_server, &gw);
    }



    PicoLightShow::WwwServer::Init();

    while (true) {
        cyw43_arch_poll();

        sleep_ms(10); 
    }

    // http_server_deinit(&http_server);
    // dns_server_deinit(&dns_server);
    // dhcp_server_deinit(&dhcp_server);
    cyw43_arch_deinit();
}
