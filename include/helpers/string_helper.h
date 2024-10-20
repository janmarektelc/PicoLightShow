#ifndef PICO_LIGHT_SHOW_STRING_HELPER_H_
#define PICO_LIGHT_SHOW_STRING_HELPER_H_

namespace PicoLightShow
{
    class StringHelper
    {
    public:
        StringHelper() = delete;
        static std::vector<std::string> Split(std::string str, char delim);
    };

} // namespace PicoLightShow
#endif // PICO_LIGHT_SHOW_STRING_HELPER_H_