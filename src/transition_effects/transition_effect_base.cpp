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

} // namespace PicoLightShow
