#include "include/dpp/dpp.h"
#include "include/light_show_runner/light_show_runner.h"
#include "include/persistent_settings/persistent_settings.h"

#include "pico/stdlib.h"

// lwIP sockets
#include "lwip/sockets.h"
#include "lwip/inet.h"
#include <fcntl.h>
#include <string.h>

namespace PicoLightShow
{
    static int dpp_sock = -1;

    bool DPP::Init()
    {
        if (dpp_sock >= 0)
            return true;

        dpp_sock = lwip_socket(AF_INET, SOCK_DGRAM, 0);
        if (dpp_sock < 0)
            return false;

        int reuse = 1;
        lwip_setsockopt(dpp_sock, SOL_SOCKET, SO_REUSEADDR, &reuse, sizeof(reuse));

        struct sockaddr_in addr;
        memset(&addr, 0, sizeof(addr));
        addr.sin_family = AF_INET;
        addr.sin_port = htons(5568);
        addr.sin_addr.s_addr = INADDR_ANY;

        if (lwip_bind(dpp_sock, (struct sockaddr *)&addr, sizeof(addr)) < 0)
        {
            lwip_close(dpp_sock);
            dpp_sock = -1;
            return false;
        }

        int flags = lwip_fcntl(dpp_sock, F_GETFL, 0);
        lwip_fcntl(dpp_sock, F_SETFL, flags | O_NONBLOCK);

        return true;
    }

    void DPP::Poll()
    {
        if (dpp_sock < 0)
            return;

        uint8_t buf[1536];
        struct sockaddr_in src;
        socklen_t srclen = sizeof(src);
        int len = lwip_recvfrom(dpp_sock, buf, sizeof(buf), 0, (struct sockaddr *)&src, &srclen);
        if (len <= 0)
            return;

        // Map raw payload as 3 bytes per LED (R,G,B)
        size_t want = (size_t)PersistentSettings::Settings.LedCount * 3;
        size_t copyLen = (size_t)len;
        if (copyLen > want)
            copyLen = want;

        LightShowRunner::PushExternalFrame(buf, copyLen);
    }

}
