#include "lwip/apps/httpd.h"

#include "include/www_server/ssi.h"
#include "include/www_server/cgi.h"

#include "include/www_server/www_server.h"

namespace PicoLightShow
{
    void WwwServer::Init()
    {
        httpd_init();
        ssi_init();
        cgi_init();
    }
} // namespace PicoLightShow
