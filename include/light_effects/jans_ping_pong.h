#ifndef PICO_LIGHT_SHOW_JANS_PING_PONG_H_
#define PICO_LIGHT_SHOW_JANS_PING_PONG_H_

#include "light_effect_base.h"

namespace PicoLightShow
{

    class JansPingPong : public PicoLightShow::LightEffectBase
    {
    public:
        JansPingPong();
        virtual ~JansPingPong();
        virtual void Draw();
        virtual void Init();
        virtual void MoveTimeFrame();
        virtual void SetProperty(char* name, char* value);
        virtual std::string GetConfigurationString();
    };

} // namespace PicoLightShow

#endif // PICO_LIGHT_SHOW_JANS_PING_PONG_H_