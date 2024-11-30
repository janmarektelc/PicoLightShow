#ifndef PICO_LIGHT_SHOW_LIGHT_SHOW_RUNNER_H_
#define PICO_LIGHT_SHOW_LIGHT_SHOW_RUNNER_H_

#include <vector>
#include <string>

#include "include/light_effects/light_effect_base.h"

namespace PicoLightShow
{
    struct LighShowEffectDescriptor
    {
        std::string EffectName;
        std::string SetupPage;
        LightEffectBase *(*GetInstance)();
        std::string Parameters;
    };

    class LightShowRunner
    {
    public:
        LightShowRunner() = delete;
        static void Init();
        static void Pool();
        static void Start();
        static void Stop();
        static bool GetIsRunning();
        static uint32_t GetFrameDelay();
        static void SetFrameDelay(uint32_t frameDelay);
        static uint32_t GetLedCount();
        static void SetLedCount(uint32_t ledCount);
        static uint8_t GetBrightness();
        static void SetBrightness(uint8_t brightness);
        static uint32_t GetEffect();
        static void SetEffect(uint32_t effect);
        static std::vector<std::string> GetEffectNames();
        static std::vector<std::string> GetEffectSetupPages();
        static void SetEffectProperty(char* name, char* value);
        static std::string GetEffectConfigurationString();

    private:
        static LightEffectBase *currentLightEffect;
        static LighShowEffectDescriptor LighShowEffectDescriptors[];
        static void SetEffectConfigurationString(std::string config);

        static LightEffectBase *CreateCustomPattern();
        static LightEffectBase *CreateColorChange();
        static LightEffectBase *CreateJansPingPong();
    };

} // namespace PicoLightShow

#endif // PICO_LIGHT_SHOW_LIGHT_SHOW_RUNNER_H_
