#include "include/transition_effects/sparkle_transition_effect.h"
#include "include/persistent_settings/persistent_settings.h"
#include <random>
#include <algorithm>

namespace PicoLightShow
{

    SparkleTransitionEffect::SparkleTransitionEffect()
    {
        this->progress = 0.0f;
    }

    SparkleTransitionEffect::~SparkleTransitionEffect()
    {
    }

    void SparkleTransitionEffect::Init()
    {
        this->progress = 0.0f;

        // create a vectore of indices from 0 to ledCount - 1, resize the vector if ledCount has changed
        if (randomizedIndices.size() != PersistentSettings::Settings.LedCount) {
            randomizedIndices.resize(PersistentSettings::Settings.LedCount);
        }
        
        std::iota(randomizedIndices.begin(), randomizedIndices.end(), 0);

        static std::random_device rd;
        static std::mt19937 g(rd());
        std::shuffle(randomizedIndices.begin(), randomizedIndices.end(), g);
    }

    bool SparkleTransitionEffect::ApplyIn(std::vector<uint32_t>* buffer, uint32_t deltaMs)
    {
        progress += static_cast<float>(deltaMs) / static_cast<float>(durationMs);
        if (progress > 1.0f) progress = 1.0f;

        size_t visibleCount = static_cast<size_t>(buffer->size() * progress);

        for (size_t i = visibleCount; i < randomizedIndices.size(); ++i) {
            size_t ledIndex = randomizedIndices[i];
            (*buffer)[ledIndex] = 0x000000;
        }

        return progress >= 1.0f; 
    }

    bool SparkleTransitionEffect::ApplyOut(std::vector<uint32_t>* buffer, uint32_t deltaMs)
    {
        progress += static_cast<float>(deltaMs) / static_cast<float>(durationMs);
        if (progress > 1.0f) progress = 1.0f;

        size_t visibleCount = static_cast<size_t>(buffer->size() * (1.0f - progress));

        for (size_t i = visibleCount; i < randomizedIndices.size(); ++i) {
            size_t ledIndex = randomizedIndices[i];
            (*buffer)[ledIndex] = 0x000000;
        }

        return progress >= 1.0f;
    }
} // namespace PicoLightShow
