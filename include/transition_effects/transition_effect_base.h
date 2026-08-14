#ifndef PICO_LIGHT_SHOW_LIGHT_TRANSITION_EFFECT_BASE_H_
#define PICO_LIGHT_SHOW_LIGHT_TRANSITION_EFFECT_BASE_H_

#include <stdint.h>
#include <string>

#include "include/types.h"
#include <vector>

namespace PicoLightShow
{

    class TransitionEffectBase
    {
    public:
        TransitionEffectBase();
        virtual ~TransitionEffectBase();
        virtual void Init() = 0;
        virtual bool ApplyIn(std::vector<uint32_t>* buffer, uint32_t deltaMs) = 0;
        virtual bool ApplyOut(std::vector<uint32_t>* buffer, uint32_t deltaMs) = 0;

    protected:
        float progress;
        uint32_t durationMs;
    };
} // namespace PicoLightShow

#endif // PICO_LIGHT_SHOW_LIGHT_TRANSITION_EFFECT_BASE_H_
