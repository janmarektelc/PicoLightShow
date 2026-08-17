#include <stdio.h>


#include "include/transition_effects/transition_effect_base.h"

namespace PicoLightShow
{

    TransitionEffectBase::TransitionEffectBase()
    {
        this->durationMs = 800; // Default duration for the wipe transition effect
        this->progress = 0.0f;
    }

    TransitionEffectBase::~TransitionEffectBase()
    {
    }

    void TransitionEffectBase::SetDuration(uint32_t durationMs)
    {
        this->durationMs = durationMs;
    }

    uint32_t TransitionEffectBase::GetDuration()
    {
        return this->durationMs;
    }

} // namespace PicoLightShow
