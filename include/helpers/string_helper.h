#ifndef PICO_LIGHT_SHOW_STRING_HELPER_H_
#define PICO_LIGHT_SHOW_STRING_HELPER_H_

#include <string>
#include <vector>

#include "include/types.h"

namespace PicoLightShow
{
    class StringHelper
    {
    public:
        StringHelper() = delete;
        static std::vector<std::string> Split(std::string str, char delim);
        static std::string ColorsToString(std::vector<Color> colors);
        static uint32_t HexStringToUint32(std::string hexValue);
    };

} // namespace PicoLightShow
#endif // PICO_LIGHT_SHOW_STRING_HELPER_H_