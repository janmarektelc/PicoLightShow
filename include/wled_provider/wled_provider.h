#pragma once
#include <string>
#include <cstdint>
#include "include/www_server/models.h"

namespace PicoLightShow
{
    class WLEDProvider {
    private:
        WLEDProvider() = delete;
    public:
        static std::string HandleGetState();
        static HttpResponse HandleSetJsonState(std::string data);;

        static std::string HandleGetInfo();

        static std::string HandleGetJson();

        static std::string HandleGetPresets();

        static void HandlePostState(const std::string& post_body);
    };
}// namespace PicoLightShow
