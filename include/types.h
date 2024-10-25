#ifndef PICO_LIGHT_SHOW_TYPES_H_
#define PICO_LIGHT_SHOW_TYPES_H_

#include <stdint.h>

namespace PicoLightShow
{

    struct Color
    {
        uint8_t Red;
        uint8_t Green;
        uint8_t Blue;

        Color(uint8_t red, uint8_t green, uint8_t blue)
            : Red(red), Green(green), Blue(blue) {}

        Color(uint32_t color)
            : Red((color & 0x00ff0000) >> 16), Green((color & 0x0000ff00) >> 8), Blue(color & 0x000000ff) {}

    };

} // namespace PicoLightShow

#endif // PICO_LIGHT_SHOW_TYPES_H_
