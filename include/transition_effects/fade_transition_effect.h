#ifndef PICO_LIGHT_SHOW_LIGHT_FADE_TRANSITION_EFFECT_BASE_H_
#define PICO_LIGHT_SHOW_LIGHT_FADE_TRANSITION_EFFECT_BASE_H_

#include "include/transition_effects/transition_effect_base.h"

namespace PicoLightShow
{
    class FadeTransitionEffect : public TransitionEffectBase
    {
    public:
        FadeTransitionEffect();
        virtual ~FadeTransitionEffect();
        virtual void Init();
        virtual bool ApplyIn(std::vector<uint32_t>* buffer, uint32_t deltaMs);
        virtual bool ApplyOut(std::vector<uint32_t>* buffer, uint32_t deltaMs);

    private:
        void ApplyAlpha(std::vector<uint32_t>* buffer, float alpha);
    };
} // namespace PicoLightShow

#endif // PICO_LIGHT_SHOW_LIGHT_FADE_TRANSITION_EFFECT_BASE_H_
