#ifndef PICO_LIGHT_SHOW_COLOR_CHANGE_H_
#define PICO_LIGHT_SHOW_COLOR_CHANGE_H_

#include <stdio.h>
#include <vector>

#include "light_effect_base.h"

namespace PicoLightShow
{

    class ColorChange : public PicoLightShow::LightEffectBase
    {
    public:
        ColorChange();
        virtual ~ColorChange();
        virtual void Draw();
        virtual void Init();
        virtual void MoveTimeFrame();
        virtual void SetProperty(char* name, char* value);
        virtual std::string GetConfigurationString();

        std::vector<Color> Colors;
    private:
        void CalculateDeltaColor();
        const uint8_t TimeFrameSize = 100;
        uint8_t colorIndex = 0;
        float DeltaRed = 0;
        float DeltaGreen = 0;
        float DeltaBlue = 0;

        float Red = 0;
        float Green = 0;
        float Blue = 0;        
    };

} // namespace PicoLightShow

#endif // PICO_LIGHT_SHOW_COLOR_CHANGE_H_