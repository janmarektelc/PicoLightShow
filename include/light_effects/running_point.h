#ifndef PICO_LIGHT_SHOW_RUNNING_POINT_H_
#define PICO_LIGHT_SHOW_RUNNING_POINT_H_

#include "light_effect_base.h"

namespace PicoLightShow
{

    class RunningPoint : public PicoLightShow::LightEffectBase
    {
    public:
        RunningPoint();
        virtual ~RunningPoint();
        virtual void Draw();
        virtual void Init();
        virtual void MoveTimeFrame();
        virtual void SetProperty(char* name, char* value);
    };

} // namespace PicoLightShow

#endif // PICO_LIGHT_SHOW_RUNNING_POINT_H_
