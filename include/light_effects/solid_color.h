#ifndef PICO_LIGHT_SHOW_SOLID_COLOR_H_
#define PICO_LIGHT_SHOW_SOLID_COLOR_H_

#include "light_effect_base.h"

namespace PicoLightShow
{

    class SolidColor : public PicoLightShow::LightEffectBase
    {
    public:
        SolidColor();
        virtual ~SolidColor();
        virtual void Draw(std::vector<uint32_t>* buffer);
        virtual void Init();
        virtual void MoveTimeFrame();
        virtual void SetProperty(const char* name, const char* value);
        virtual std::string GetConfigurationString();
    private:
        Color CurrentColor = {0, 0, 0};
    };

} // namespace PicoLightShow

#endif // PICO_LIGHT_SHOW_SOLID_COLOR_H_
