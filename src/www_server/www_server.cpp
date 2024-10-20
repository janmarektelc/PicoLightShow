#include "lwip/apps/httpd.h"
#include "pico/cyw43_arch.h"

#include "include/www_server/ssi.h"
#include "include/www_server/cgi.h"
#include "include/persistent_settings/persistent_settings.h"

#include "include/www_server/www_server.h"

static void *current_connection;
err_t httpd_post_begin(void *connection, const char *uri, const char *http_request,
        u16_t http_request_len, int content_len, char *response_uri,
        u16_t response_uri_len, u8_t *post_auto_wnd)
{
    if (memcmp(uri, "/network_data", 13) == 0 && current_connection != connection) {
        current_connection = connection;
        snprintf(response_uri, response_uri_len, "/fail.html");
        *post_auto_wnd = 1;
        return ERR_OK;
    }
    return ERR_VAL;
}

err_t httpd_post_receive_data(void *connection, struct pbuf *p)
{
    err_t ret = ERR_VAL;
    LWIP_ASSERT("NULL pbuf", p != NULL);
    if (current_connection == connection) {
        ret = ERR_OK;
    }
    //expected data format
    //~wifiMode(ap=0/client=1)~ssid~passwd~isdhcp(0/1)~ipAddress~ipMask~gwIp~
    PicoLightShow::PersistentSettings::SetByConfigString(std::string((char*)p->payload, p->len));
    pbuf_free(p);
    PicoLightShow::PersistentSettings::Save();
    return ret;
}

void httpd_post_finished(void *connection, char *response_uri, u16_t response_uri_len)
{
    snprintf(response_uri, response_uri_len, "/fail.html");
    if (current_connection == connection) {
        snprintf(response_uri, response_uri_len, "/success.html");
    }
    current_connection = NULL;
}

namespace PicoLightShow
{
    void WwwServer::Init()
    {
        httpd_init();
        ssi_init();
        cgi_init();
    }
} // namespace PicoLightShow
