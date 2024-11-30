#ifndef PICO_LIGHT_SHOW_CUSTOM_PATTERN_H_
#define PICO_LIGHT_SHOW_CUSTOM_PATTERN_H_

#include <stdio.h>
#include <vector>

#include "light_effect_base.h"

namespace PicoLightShow
{
    enum PatternDrawKind
    {
        Once,
        Repeat,
        Stretch,
    };

    class CustomPattern : public PicoLightShow::LightEffectBase
    {
    public:
        CustomPattern();
        virtual ~CustomPattern();
        virtual void Draw();
        virtual void Init();
        virtual void MoveTimeFrame();
        virtual void SetProperty(const char* name, const char* value);
        virtual std::string GetConfigurationString();
        std::vector<Color> ColorPattern;
        int8_t Direction;
        bool PingPong;
        PatternDrawKind DrawKind;

    protected:
        virtual void DrawRepeat();
        virtual void DrawStretch();
        virtual void DrawOnce();
        virtual void MoveTimeFrameRepeat();
        virtual void MoveTimeFrameStretch();
        virtual void MoveTimeFrameOnce();
    };

} // namespace PicoLightShow

#endif // PICO_LIGHT_SHOW_CUSTOM_PATTERN_H_
