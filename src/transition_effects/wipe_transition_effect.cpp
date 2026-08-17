#include "include/transition_effects/wipe_transition_effect.h"

namespace PicoLightShow
{
    WipeTransitionEffect::WipeTransitionEffect(WipeStyle style)
    {
        this->progress = 0.0f;
        this->style = style;
    }

    WipeTransitionEffect::~WipeTransitionEffect()
    {
    }

    void WipeTransitionEffect::Init()
    {
        this->progress = 0.0f;
    }

    bool WipeTransitionEffect::ApplyIn(std::vector<uint32_t>* buffer, uint32_t deltaMs)
    {
        progress += static_cast<float>(deltaMs) / static_cast<float>(durationMs);
        if (progress > 1.0f) progress = 1.0f;

        MaskBuffer(buffer, progress);

        return progress >= 1.0f;
    }

    bool WipeTransitionEffect::ApplyOut(std::vector<uint32_t>* buffer, uint32_t deltaMs)
    {
        progress += static_cast<float>(deltaMs) / static_cast<float>(durationMs);
        if (progress > 1.0f) progress = 1.0f;

        MaskBuffer(buffer, 1.0f - progress);

        return progress >= 1.0f;
    }

    void WipeTransitionEffect::MaskBuffer(std::vector<uint32_t>* buffer, float visibleRatio) {
        size_t totalLeds = buffer->size();
        if (totalLeds == 0) return;

        switch (style) {
            case WipeStyle::FromStart: {
                size_t visibleLeds = static_cast<size_t>(totalLeds * visibleRatio);
                for (size_t i = visibleLeds; i < totalLeds; ++i) {
                    (*buffer)[i] = 0x000000;
                }
                break;
            }

            case WipeStyle::FromEnd: {
                size_t visibleLeds = static_cast<size_t>(totalLeds * visibleRatio);
                size_t hiddenLeds = totalLeds - visibleLeds;
                for (size_t i = 0; i < hiddenLeds; ++i) {
                    (*buffer)[i] = 0x000000;
                }
                break;
            }

            case WipeStyle::FromCenter: {
                size_t visibleLeds = static_cast<size_t>(totalLeds * visibleRatio);
                size_t halfTotal = totalLeds / 2;
                size_t halfVisible = visibleLeds / 2;

                size_t startIdx = (halfTotal > halfVisible) ? (halfTotal - halfVisible) : 0;
                size_t endIdx = std::min(totalLeds, halfTotal + halfVisible);

                for (size_t i = 0; i < startIdx; ++i) {
                    (*buffer)[i] = 0x000000;
                }
                for (size_t i = endIdx; i < totalLeds; ++i) {
                    (*buffer)[i] = 0x000000;
                }
                break;
            }

            case WipeStyle::ToCenter: {
                size_t visibleLeds = static_cast<size_t>(totalLeds * visibleRatio);
                size_t hiddenLeds = totalLeds - visibleLeds;
                
                size_t halfTotal = totalLeds / 2;
                size_t halfHidden = hiddenLeds / 2;

                size_t hideStartIdx = (halfTotal > halfHidden) ? (halfTotal - halfHidden) : 0;
                size_t hideEndIdx = std::min(totalLeds, halfTotal + halfHidden + (hiddenLeds % 2));

                for (size_t i = hideStartIdx; i < hideEndIdx; ++i) {
                    (*buffer)[i] = 0x000000;
                }
                break;
            }
        }
    }
} // namespace PicoLightShow
