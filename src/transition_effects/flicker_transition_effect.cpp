#include "include/transition_effects/flicker_transition_effect.h"
#include "include/persistent_settings/persistent_settings.h"
#include <random>
#include <algorithm>
#include "pico/time.h"

namespace PicoLightShow
{

    FlickerTransitionEffect::FlickerTransitionEffect()
    {
        this->progress = 0.0f;
        this->rng.seed(time_us_32());
    }

    FlickerTransitionEffect::~FlickerTransitionEffect()
    {
    }

    void FlickerTransitionEffect::Init()
    {
        this->progress = 0.0f;
        this->rng.seed(time_us_32());
    }

    bool FlickerTransitionEffect::ApplyIn(std::vector<uint32_t>* buffer, uint32_t deltaMs)
    {
        progress += static_cast<float>(deltaMs) / static_cast<float>(durationMs);
        if (progress > 1.0f) progress = 1.0f;

        float threshold = progress * progress;
        float randomVal = dist(rng);

        if (randomVal > threshold) {
            DimBuffer(buffer, 0.05f); 
        } 

        return progress >= 1.0f;
    }

    bool FlickerTransitionEffect::ApplyOut(std::vector<uint32_t>* buffer, uint32_t deltaMs)
    {
        progress += static_cast<float>(deltaMs) / static_cast<float>(durationMs);
        if (progress > 1.0f) progress = 1.0f;

        float threshold = (1.0f - progress) * (1.0f - progress);
        float randomVal = dist(rng);

        if (randomVal > threshold) {
            DimBuffer(buffer, 0.0f);
        }

        return progress >= 1.0f;
    }

    void FlickerTransitionEffect::DimBuffer(std::vector<uint32_t>* buffer, float alpha) 
    {
        for (size_t i = 0; i < buffer->size(); ++i) {
            if (alpha == 0.0f) {
                (*buffer)[i] = 0x000000;
            } else {
                uint32_t color = (*buffer)[i];
                uint8_t r = static_cast<uint8_t>(((color >> 16) & 0xFF) * alpha);
                uint8_t g = static_cast<uint8_t>(((color >> 24) & 0xFF) * alpha);
                uint8_t b = static_cast<uint8_t>(((color >> 8)  & 0xFF) * alpha);

                (*buffer)[i] = (static_cast<uint32_t>(g) << 24) |
                                (static_cast<uint32_t>(r) << 16) |
                                (static_cast<uint32_t>(b) << 8);
            }
        }
    }
} // namespace PicoLightShow
