#ifndef PICO_LIGHT_SHOW_DPP_H_
#define PICO_LIGHT_SHOW_DPP_H_

#include <stdint.h>

namespace PicoLightShow
{
    class DPP
    {
    public:
        DPP() = delete;
        static bool Init();
        static void Poll();
    };
}

#endif // PICO_LIGHT_SHOW_DPP_H_
