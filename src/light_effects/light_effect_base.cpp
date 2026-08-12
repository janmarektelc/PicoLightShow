#include <stdio.h>
#include "hardware/pio.h"
#include "pico/stdlib.h"

#include "include/light_effects/light_effect_base.h"

namespace PicoLightShow
{

    LightEffectBase::LightEffectBase()
    {
        this->time = 0;
    }

    LightEffectBase::~LightEffectBase()
    {
    }

    uint32_t LightEffectBase::EncodeColor(uint8_t r, uint8_t g, uint8_t b) 
    {
        return (((uint32_t)r << 8) | ((uint32_t)g << 16) | (uint32_t)b) << 8u;
    }

    uint32_t LightEffectBase::EncodeColor(Color color) 
    {
        return (((uint32_t)color.Red << 8) | ((uint32_t)color.Green << 16) | (uint32_t)color.Blue) << 8u;
    }

    uint32_t LightEffectBase::GetLedCount()
    {
        return this->ledCount;
    }

    void LightEffectBase::SetLedCount(uint32_t ledCount)
    {
        this->ledCount = ledCount;
    }

} // namespace PicoLightShow
