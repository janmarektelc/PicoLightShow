#include "include/transition_effects/fade_transition_effect.h"
#include "include/persistent_settings/persistent_settings.h"
#include <algorithm>

namespace PicoLightShow
{

    FadeTransitionEffect::FadeTransitionEffect()
    {
        this->progress = 0.0f;
    }

    FadeTransitionEffect::~FadeTransitionEffect()
    {
    }

    void FadeTransitionEffect::Init()
    {
        this->progress = 0.0f;
    }

    bool FadeTransitionEffect::ApplyIn(std::vector<uint32_t>* buffer, uint32_t deltaMs)
    {
        progress += static_cast<float>(deltaMs) / static_cast<float>(durationMs);
        if (progress > 1.0f) progress = 1.0f;

        ApplyAlpha(buffer, progress);

        return progress >= 1.0f;
    }

    bool FadeTransitionEffect::ApplyOut(std::vector<uint32_t>* buffer, uint32_t deltaMs)
    {
        progress += static_cast<float>(deltaMs) / static_cast<float>(durationMs);
        if (progress > 1.0f) progress = 1.0f;

        ApplyAlpha(buffer, 1.0f - progress);

        return progress >= 1.0f;
    }

    void FadeTransitionEffect::ApplyAlpha(std::vector<uint32_t>* buffer, float alpha)
    {
        alpha = std::clamp(alpha, 0.0f, 1.0f);

        for (size_t i = 0; i < buffer->size(); ++i)
        {
            uint32_t color = (*buffer)[i];

            uint8_t r = (color >> 16) & 0xFF;
            uint8_t g = (color >> 24) & 0xFF;
            uint8_t b = (color >> 8)  & 0xFF;

            r = static_cast<uint8_t>(r * alpha);
            g = static_cast<uint8_t>(g * alpha);
            b = static_cast<uint8_t>(b * alpha);

            (*buffer)[i] = (static_cast<uint32_t>(g) << 24) |
                           (static_cast<uint32_t>(r) << 16) |
                           (static_cast<uint32_t>(b) << 8);
        }
    }
} // namespace PicoLightShow
