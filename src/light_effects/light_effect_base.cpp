#include <stdio.h>
#include "hardware/pio.h"
#include "pico/stdlib.h"

#include "include/light_effects/light_effect_base.h"

namespace PicoLightShow
{

    LightEffectBase::LightEffectBase()
    {
        this->time = 0;
        this->fbrightness = 1.0;
        this->brightness = 255;
    }

    LightEffectBase::~LightEffectBase()
    {
    }

    void LightEffectBase::PutPixel(uint8_t r, uint8_t g, uint8_t b)
    {
        pio_sm_put_blocking(WS_PIO_INSTANCE, WS_STATE_MACHINE_INDEX,
                            (((uint32_t)(r * fbrightness) << 8) |
                             ((uint32_t)(g * fbrightness) << 16) |
                             (uint32_t)(b * fbrightness))
                                << 8u);
    }

    void LightEffectBase::PutPixel(Color color)
    {
        pio_sm_put_blocking(WS_PIO_INSTANCE, WS_STATE_MACHINE_INDEX,
                            (((uint32_t)(color.Red * fbrightness) << 8) |
                             ((uint32_t)(color.Green * fbrightness) << 16) |
                             (uint32_t)(color.Blue * fbrightness))
                                << 8u);
    }

    uint8_t LightEffectBase::GetBrightness()
    {
        return brightness;
    }

    void LightEffectBase::SetBrightness(uint8_t brightness)
    {
        fbrightness = ((float)brightness) / 255;
        this->brightness = brightness;
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
