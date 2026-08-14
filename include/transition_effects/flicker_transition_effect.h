#ifndef PICO_LIGHT_SHOW_LIGHT_FLICKER_TRANSITION_EFFECT_BASE_H_
#define PICO_LIGHT_SHOW_LIGHT_FLICKER_TRANSITION_EFFECT_BASE_H_

#include "include/transition_effects/transition_effect_base.h"
#include <random>

namespace PicoLightShow
{
    class FlickerTransitionEffect : public TransitionEffectBase
    {
    public:
        FlickerTransitionEffect();
        virtual ~FlickerTransitionEffect();
        virtual void Init();
        virtual bool ApplyIn(std::vector<uint32_t>* buffer, uint32_t deltaMs);
        virtual bool ApplyOut(std::vector<uint32_t>* buffer, uint32_t deltaMs);

    private:
        std::minstd_rand rng;
        std::uniform_real_distribution<float> dist;

        void DimBuffer(std::vector<uint32_t>* buffer, float alpha);
    };
} // namespace PicoLightShow

#endif // PICO_LIGHT_SHOW_LIGHT_FLICKER_TRANSITION_EFFECT_BASE_H_
