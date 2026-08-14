#ifndef PICO_LIGHT_SHOW_LIGHT_SPARKLE_TRANSITION_EFFECT_BASE_H_
#define PICO_LIGHT_SHOW_LIGHT_SPARKLE_TRANSITION_EFFECT_BASE_H_

#include "include/transition_effects/transition_effect_base.h"

namespace PicoLightShow
{
    class SparkleTransitionEffect : public TransitionEffectBase
    {
    public:
        SparkleTransitionEffect();
        virtual ~SparkleTransitionEffect();
        virtual void Init();
        virtual bool ApplyIn(std::vector<uint32_t>* buffer, uint32_t deltaMs);
        virtual bool ApplyOut(std::vector<uint32_t>* buffer, uint32_t deltaMs);

    private:
        std::vector<size_t> randomizedIndices;
    };
} // namespace PicoLightShow

#endif // PICO_LIGHT_SHOW_LIGHT_SPARKLE_TRANSITION_EFFECT_BASE_H_
