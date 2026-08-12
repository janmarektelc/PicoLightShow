#ifndef PICO_LIGHT_SHOW_LIGHT_EFFECT_BASE_H_
#define PICO_LIGHT_SHOW_LIGHT_EFFECT_BASE_H_

#include <stdint.h>
#include <string>

#include "include/types.h"
#include "include/constants.h"
#include <vector>

namespace PicoLightShow
{

    class LightEffectBase
    {
    public:
        LightEffectBase();
        virtual ~LightEffectBase();
        virtual void Draw(std::vector<uint32_t>* buffer) = 0;
        virtual void Init() = 0;
        virtual void MoveTimeFrame() = 0;
        virtual uint32_t GetLedCount();
        virtual void SetLedCount(uint32_t ledCount);
        virtual void SetProperty(const char* name, const char* value) = 0;
        virtual std::string GetConfigurationString() = 0;

    protected:
        static uint32_t EncodeColor(uint8_t r, uint8_t g, uint8_t b);
        static uint32_t EncodeColor(Color color);

        int ledCount;
        int time;
    };

} // namespace PicoLightShow

#endif // PICO_LIGHT_SHOW_LIGHT_EFFECT_BASE_H_
