#ifndef PICO_LIGHT_SHOW_LIGHT_EFFECT_BASE_H_
#define PICO_LIGHT_SHOW_LIGHT_EFFECT_BASE_H_

#include <stdint.h>

#include "include/types.h"
#include "include/constants.h"

namespace PicoLightShow
{

    class LightEffectBase
    {
    public:
        LightEffectBase();
        virtual ~LightEffectBase();
        virtual void Draw() = 0;
        virtual void Init() = 0;
        virtual void MoveTimeFrame() = 0;
        virtual uint8_t GetBrightness();
        virtual void SetBrightness(uint8_t brightness);
        virtual uint32_t GetLedCount();
        virtual void SetLedCount(uint32_t ledCount);

    protected:
        void PutPixel(uint8_t r, uint8_t g, uint8_t b);
        void PutPixel(Color color);

        int ledCount;
        int time;
        uint8_t brightness;
    
    private:
        float fbrightness;
    };

} // namespace PicoLightShow

#endif // PICO_LIGHT_SHOW_LIGHT_EFFECT_BASE_H_
