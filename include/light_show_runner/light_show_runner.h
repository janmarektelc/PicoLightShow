#ifndef PICO_LIGHT_SHOW_LIGHT_SHOW_RUNNER_H_
#define PICO_LIGHT_SHOW_LIGHT_SHOW_RUNNER_H_

#include <vector>
#include <string>

#include "include/light_effects/light_effect_base.h"
#include <include/transition_effects/transition_effect_base.h>
#include <functional>
#include <memory>

namespace PicoLightShow
{
    struct LighShowEffectDescriptor
    {
        std::string EffectName;
        std::string SetupPage;
        LightEffectBase *(*GetInstance)();
        std::string Parameters;
    };

    struct TransitionEffectDescriptor {
        std::string EffectName;
        std::function<std::unique_ptr<TransitionEffectBase>()> CreateInstance;
    };

    enum class LightShowRunnerState
    {
        Standby,
        Starting,
        Running,
        Stopping
    };

    class LightShowRunner
    {
    public:
        LightShowRunner() = delete;
        static void Init();
        static void Pool();
        static void Start();
        static void Stop();
        static void SwitchOn();
        static void SwitchOff();
        static bool GetIsRunning();
        static bool GetSwitchOn();
        static uint8_t GetFrameDelay();
        static void SetFrameDelay(uint8_t frameDelay);
        static uint32_t GetLedCount();
        static void SetLedCount(uint32_t ledCount);
        static uint8_t GetBrightness();
        static void SetBrightness(uint8_t brightness);
        static uint32_t GetEffect();
        static void SetEffect(uint32_t effect);
        static std::vector<std::string> GetEffectNames();
        static uint8_t GetEffectsCount();
        static std::vector<std::string> GetEffectSetupPages();
        static void SetEffectProperty(const char* name, const char* value);
        static std::string GetEffectConfigurationString();
        static void SetSolidColor(uint8_t r, uint8_t g, uint8_t b);
        static Color GetSolidColor();
        static uint32_t GetTransitionEffect();
        static void SetTransitionEffect(uint32_t effect);
        static std::vector<std::string> GetTransitionEffectNames();
        
        //callback for state change
        using OnStateChangedCallback = std::function<void()>;
        static void SetOnStateChangedCallback(OnStateChangedCallback cb);

    private:
        static LightShowRunnerState state;
        static bool startAnimationDone;
        static bool stopAnimationDone;
        static uint64_t lastFrameTimeUs;
        static uint32_t timeAccumulatorMs;
        static bool isSwitchOn;
        static std::vector<uint32_t>* ledBuffer;
        static LightEffectBase *currentLightEffect;
        static uint32_t solidColorEffectIndex;
        static std::unique_ptr<TransitionEffectBase> currentTransitionEffect;
        static LighShowEffectDescriptor LighShowEffectDescriptors[];
        static TransitionEffectDescriptor TransitionEffectDescriptors[];
        static void SetEffectConfigurationString(std::string config);

        static void MoveStateMachine();
        static void RenderEffectFrame(uint32_t deltaMs);
        
        static int frame;

        static LightEffectBase *CreateCustomPattern();
        static LightEffectBase *CreateColorChange();
        static LightEffectBase *CreateRunningPoint();
        static LightEffectBase *CreateSolidColor();

        // callback for state change
        static OnStateChangedCallback s_onStateChanged;
        static void NotifyStateChanged();

        static void PutPixel(uint8_t r, uint8_t g, uint8_t b);
        static void PutPixel(Color color);
    };

} // namespace PicoLightShow

#endif // PICO_LIGHT_SHOW_LIGHT_SHOW_RUNNER_H_
