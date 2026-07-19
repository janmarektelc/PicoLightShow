#pragma once
#include "include/www_server/www_server.h"
#include "models.h"

namespace PicoLightShow
{
    class LightShowWeb : public WwwServer {
    public:
        HttpResponse onRequest(const HttpRequest& req) override;
    };
}// namespace PicoLightShow