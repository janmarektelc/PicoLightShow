#ifndef PICO_LIGHT_SHOW_LIGHT_WIPE_TRANSITION_EFFECT_BASE_H_
#define PICO_LIGHT_SHOW_LIGHT_WIPE_TRANSITION_EFFECT_BASE_H_

#include "include/transition_effects/transition_effect_base.h"

namespace PicoLightShow
{
    enum class WipeStyle {
        FromStart,
        FromEnd,
        FromCenter,
        ToCenter
    };

    class WipeTransitionEffect : public TransitionEffectBase
    {
    public:
        WipeTransitionEffect();
        virtual ~WipeTransitionEffect();
        virtual void Init();
        virtual bool ApplyIn(std::vector<uint32_t>* buffer, uint32_t deltaMs);
        virtual bool ApplyOut(std::vector<uint32_t>* buffer, uint32_t deltaMs);

    private:
        WipeStyle style;

        void MaskBuffer(std::vector<uint32_t>* buffer, float visibleRatio);
    };
} // namespace PicoLightShow

#endif // PICO_LIGHT_SHOW_LIGHT_WIPE_TRANSITION_EFFECT_BASE_H_
