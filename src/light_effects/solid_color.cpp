#include "include/light_effects/solid_color.h"
#include "include/helpers/string_helper.h"
#include <cstring>

namespace PicoLightShow
{

    SolidColor::SolidColor()
    {
    }

    SolidColor::~SolidColor()
    {
    }

    void SolidColor::Draw(std::vector<uint32_t>* buffer)
    {
        std::fill(buffer->begin(), buffer->end(), EncodeColor(CurrentColor));
    }

    void SolidColor::MoveTimeFrame()
    {
    }

    void SolidColor::Init()
    {
    }

    void SolidColor::SetProperty(const char* name, const char* value)
    {
        if (strcmp(name, "color") == 0)
        {
            CurrentColor = Color(StringHelper::HexStringToUint32(value));
            Init();
        }
    }

    std::string SolidColor::GetConfigurationString()
    {
        return "color="+StringHelper::ColorToString(CurrentColor);
    }

} // namespace PicoLightShow
