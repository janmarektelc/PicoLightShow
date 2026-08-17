#include <stdio.h>

#include "pico/stdlib.h"
#include "hardware/pio.h"
#include "ws2812.pio.h"

#include "include/light_effects/running_point.h"
#include "include/light_effects/custom_pattern.h"
#include "include/light_effects/color_change.h"
#include "include/light_effects/solid_color.h"
#include "include/transition_effects/sparkle_transition_effect.h"
#include "include/transition_effects/wipe_transition_effect.h"
#include "include/persistent_settings/persistent_settings.h"
#include "include/transition_effects/flicker_transition_effect.h"
#include "include/transition_effects/fade_transition_effect.h"
#include "include/helpers/string_helper.h"

#include "include/light_show_runner/light_show_runner.h"
#include "include/DDP/ddp.h"

namespace PicoLightShow
{
    LighShowEffectDescriptor LightShowRunner::LighShowEffectDescriptors[] = {
        {"Solid color", "single_color_setup.shtml", CreateSolidColor, "color=ffffff"},
        {"Running point", "custom_pattern_setup.shtml", CreateCustomPattern, "draw-kind=0&ping-pong=1&direction=1&colors=ff0000"},
        {"Running colors", "custom_pattern_setup.shtml", CreateCustomPattern, "draw-kind=2&ping-pong=1&direction=0&colors=ff0000,ffff00,00ff00,00ffff,0000ff,ff00ff"},
        {"Running colors 1", "custom_pattern_setup.shtml", CreateCustomPattern, "draw-kind=1&ping-pong=0&direction=1&colors=ff0000,ffff00,00ff00,00ffff,0000ff,ff00ff"},
        {"Snakes", "custom_pattern_setup.shtml", CreateCustomPattern, "draw-kind=2&ping-pong=0&direction=0&colors=ff0000,000000,00ff00,000000,0000ff,000000"},
        {"Color change", "change_color_setup.shtml", CreateColorChange, "colors=640000,ff0000,006400,00ff00,000064,0000ff"},
        {"Breath", "change_color_setup.shtml", CreateColorChange, "colors=640000,ff0000"},
        //{"Running point simple", "", CreateRunningPoint, ""},
    };

    LightEffectBase *LightShowRunner::currentLightEffect = nullptr;
    TransitionEffectBase *LightShowRunner::currentTransitionEffect = nullptr;
    std::vector<uint32_t>* LightShowRunner::ledBuffer = nullptr;
    LightShowRunner::OnStateChangedCallback LightShowRunner::s_onStateChanged = nullptr;
    uint32_t LightShowRunner::solidColorEffectIndex = 0;
    
    bool LightShowRunner::isSwitchOn = true;
    LightShowRunnerState LightShowRunner::state = LightShowRunnerState::Standby;
    bool LightShowRunner::startAnimationDone = false;
    bool LightShowRunner::stopAnimationDone = false;

    uint64_t LightShowRunner::lastFrameTimeUs = 0;
    uint32_t LightShowRunner::timeAccumulatorMs = 0;
    int LightShowRunner::frame = 0;

    LightEffectBase *LightShowRunner::CreateCustomPattern()
    {
        return new CustomPattern();
    }

    LightEffectBase *LightShowRunner::CreateColorChange()
    {
        return new ColorChange();
    }

    LightEffectBase *LightShowRunner::CreateRunningPoint()
    {
        return new RunningPoint();
    }

    LightEffectBase *LightShowRunner::CreateSolidColor()
    {
        return new SolidColor();
    }

    void LightShowRunner::Init()
    {
        uint offset = pio_add_program(WS_PIO_INSTANCE, &ws2812_program);
        ws2812_program_init(WS_PIO_INSTANCE, WS_STATE_MACHINE_INDEX, offset, WS2812_PIN, 800000, IS_RGBW);

        ledBuffer = new std::vector<uint32_t>(PersistentSettings::Settings.LedCount, 0);

        currentLightEffect = LighShowEffectDescriptors[PersistentSettings::Settings.EffectIndex].GetInstance();
        currentLightEffect->SetLedCount(PersistentSettings::Settings.LedCount);
        if (PersistentSettings::Settings.CurrentEffectConfiguration[0] != '\0')
        {
            LighShowEffectDescriptors[PersistentSettings::Settings.EffectIndex].Parameters = PersistentSettings::Settings.CurrentEffectConfiguration;
        }

        SetEffectConfigurationString(LighShowEffectDescriptors[PersistentSettings::Settings.EffectIndex].Parameters);

        currentLightEffect->Init();

        //currentTransitionEffect = new WipeTransitionEffect();
        //currentTransitionEffect = new SparkleTransitionEffect();
        currentTransitionEffect = new FlickerTransitionEffect();
        //currentTransitionEffect = new FadeTransitionEffect();
    }

    void LightShowRunner::Pool()
    {
        uint64_t nowUs = time_us_64();
        uint32_t deltaMs = 0;
        if (lastFrameTimeUs > 0) {
            deltaMs = static_cast<uint32_t>((nowUs - lastFrameTimeUs) / 1000);
        } else {
            deltaMs = FRAME_TIME_MS;
        }
        lastFrameTimeUs = time_us_64();;

        MoveStateMachine();

        if (!DDP::IsActive())
        {
            switch (state)
            {
                case LightShowRunnerState::Standby:
                    std::fill(ledBuffer->begin(), ledBuffer->end(), 0u);
                    frame = 0;
                    break;
                case LightShowRunnerState::Starting:
                    RenderEffectFrame(deltaMs);
                    startAnimationDone = currentTransitionEffect->ApplyIn(ledBuffer, deltaMs);
                    break;
                case LightShowRunnerState::Running:
                    RenderEffectFrame(deltaMs);
                    break;
                case LightShowRunnerState::Stopping:
                    RenderEffectFrame(deltaMs);
                    stopAnimationDone = currentTransitionEffect->ApplyOut(ledBuffer, deltaMs);
                    break;
            }

            float fbrightness = ((float)PersistentSettings::Settings.Brightness) / 255.0f;

            //put buffer to pio and apply brightness
            for (int i = 0; i < ledBuffer->size(); i++)
            {
                uint32_t color = (*ledBuffer)[i];

                uint8_t r = (color >> 16) & 0xFF;
                uint8_t g = (color >> 24) & 0xFF;
                uint8_t b = (color >> 8)  & 0xFF;

                // apply brightness
                r = (uint8_t)(r * fbrightness);
                g = (uint8_t)(g * fbrightness);
                b = (uint8_t)(b * fbrightness);

                uint32_t pioColor = (((uint32_t)r << 8) | ((uint32_t)g << 16) | (uint32_t)b) << 8u;

                pio_sm_put_blocking(WS_PIO_INSTANCE, WS_STATE_MACHINE_INDEX, pioColor);
            }
        }
        uint32_t elapsedTimeMs = static_cast<uint32_t>((time_us_64() - nowUs) / 1000);

        // sleep to maintain target FPS
        if (elapsedTimeMs < FRAME_TIME_MS) {
            sleep_ms(FRAME_TIME_MS - elapsedTimeMs);
        }
    }

    void LightShowRunner::MoveStateMachine()
    {
        switch (state)
        {
        case LightShowRunnerState::Standby:
            if (isSwitchOn)
            {
                state = LightShowRunnerState::Starting;
                startAnimationDone = false;
                currentTransitionEffect->Init();
            }
            break;
        case LightShowRunnerState::Starting:
            if (!isSwitchOn)
            {
                state = LightShowRunnerState::Standby;
            }
            if (startAnimationDone)
            {
                state = LightShowRunnerState::Running;
            }
            break;
        case LightShowRunnerState::Running:
            if (!isSwitchOn)
            {
                state = LightShowRunnerState::Stopping;
                stopAnimationDone = false;
                currentTransitionEffect->Init();
            }
            break;
        case LightShowRunnerState::Stopping:
            if (isSwitchOn)
            {
                state = LightShowRunnerState::Starting;
                startAnimationDone = false;
                currentTransitionEffect->Init();
            }
            if (stopAnimationDone)
            {
                state = LightShowRunnerState::Standby;
            }
            break;
        default:
            break;
        }
    }

    void LightShowRunner::RenderEffectFrame(uint32_t deltaMs)
    {
        //if annimation is running, move time frame  
        if (PersistentSettings::Settings.IsRunning)
        {
            uint8_t speed = PersistentSettings::Settings.Delay;

            if (speed > 0) {
                timeAccumulatorMs += deltaMs;
                uint32_t stepIntervalMs = 15 + ((255 - speed) * 85) / 254;
                while (timeAccumulatorMs >= stepIntervalMs) {
                    currentLightEffect->MoveTimeFrame();
                    timeAccumulatorMs -= stepIntervalMs;
                }
            }
        }
        //draw to the buffer
        currentLightEffect->Draw(ledBuffer);
    }

    void LightShowRunner::Start()
    {
        PersistentSettings::Settings.IsRunning = true;
        NotifyStateChanged();
    }

    void LightShowRunner::Stop()
    {
        PersistentSettings::Settings.IsRunning = false;
        NotifyStateChanged();
    }

    void LightShowRunner::SwitchOn()
    {
        currentLightEffect->Init();
        isSwitchOn = true;
        NotifyStateChanged();
    }

    void LightShowRunner::SwitchOff()
    {
        isSwitchOn = false;
        NotifyStateChanged();
    }

    bool LightShowRunner::GetIsRunning()
    {
        return PersistentSettings::Settings.IsRunning;
    }

    bool LightShowRunner::GetSwitchOn()
    {
        return isSwitchOn;
    }

    uint8_t LightShowRunner::GetFrameDelay()
    {
        return PersistentSettings::Settings.Delay;
    }

    void LightShowRunner::SetFrameDelay(uint8_t frameDelay)
    {
        if (frameDelay >= 1 && frameDelay <= 255)
        {
            PersistentSettings::Settings.Delay = frameDelay;
        }
    }

    uint32_t LightShowRunner::GetLedCount()
    {
        return PersistentSettings::Settings.LedCount;
    }

    void LightShowRunner::SetLedCount(uint32_t ledCount)
    {
        PersistentSettings::Settings.LedCount = ledCount;
        ledBuffer->resize(ledCount, 0);
        currentLightEffect->SetLedCount(ledCount);
    }

    uint8_t LightShowRunner::GetBrightness()
    {
        return PersistentSettings::Settings.Brightness;
    }

    void LightShowRunner::SetBrightness(uint8_t brightness)
    {
        PersistentSettings::Settings.Brightness = brightness;
        NotifyStateChanged();
    }

    uint32_t LightShowRunner::GetEffect()
    {
        return PersistentSettings::Settings.EffectIndex;
    }

    void LightShowRunner::SetEffect(uint32_t effect)
    {
        LighShowEffectDescriptors[PersistentSettings::Settings.EffectIndex].Parameters = currentLightEffect->GetConfigurationString();

        PersistentSettings::Settings.EffectIndex = effect;
        delete currentLightEffect;

        currentLightEffect = LighShowEffectDescriptors[effect].GetInstance();
        currentLightEffect->SetLedCount(PersistentSettings::Settings.LedCount);

        SetEffectConfigurationString(LighShowEffectDescriptors[effect].Parameters);

        currentLightEffect->Init();
        NotifyStateChanged();
    }

    std::vector<std::string> LightShowRunner::GetEffectNames()
    {
        int arraySize = sizeof(LighShowEffectDescriptors) / sizeof(LighShowEffectDescriptors[0]); 
        std::vector<std::string> vect;
        for (int i=0; i<arraySize; i++)
        {
            vect.push_back(LighShowEffectDescriptors[i].EffectName);
        }

        return vect;
    }

    uint8_t LightShowRunner::GetEffectsCount()
    {
        return sizeof(LighShowEffectDescriptors) / sizeof(LighShowEffectDescriptors[0]); 
    }

    std::vector<std::string> LightShowRunner::GetEffectSetupPages()
    {
        int arraySize = sizeof(LighShowEffectDescriptors) / sizeof(LighShowEffectDescriptors[0]); 
        std::vector<std::string> vect;
        for (int i=0; i<arraySize; i++)
        {
            vect.push_back(LighShowEffectDescriptors[i].SetupPage);
        }
        return vect;
    }

    void LightShowRunner::SetEffectProperty(const char* name, const char* value)
    {
        if (currentLightEffect)
        {
            currentLightEffect->SetProperty(name, value);
        }
    }

    std::string LightShowRunner::GetEffectConfigurationString()
    {
        if (currentLightEffect)
            return currentLightEffect->GetConfigurationString();
        
        return "";
    }

    void LightShowRunner::SetEffectConfigurationString(std::string config)
    {
        if (config.empty())
            return;

        std::vector<std::string> params = StringHelper::Split(config, '&');
        for (int i = 0; i < params.size(); i++)
        {
            std::vector<std::string> param = StringHelper::Split(params[i], '=');
            currentLightEffect->SetProperty(param[0].c_str(), param[1].c_str());
        }
    }

    void LightShowRunner::SetSolidColor(uint8_t r, uint8_t g, uint8_t b)
    {
        SetEffect(solidColorEffectIndex);
        SetEffectProperty("color", StringHelper::ColorToString(r, g, b).c_str());
    }

    Color LightShowRunner::GetSolidColor()
    {
        return Color(StringHelper::HexStringToUint32(LighShowEffectDescriptors[solidColorEffectIndex].Parameters.substr(6, 6)));
    }

    void LightShowRunner::SetOnStateChangedCallback(OnStateChangedCallback cb) {
        s_onStateChanged = cb;
    }

    void LightShowRunner::NotifyStateChanged() {
        if (s_onStateChanged) {
            s_onStateChanged();
        }
    }

    void LightShowRunner::PutPixel(uint8_t r, uint8_t g, uint8_t b)
    {
        pio_sm_put_blocking(WS_PIO_INSTANCE, WS_STATE_MACHINE_INDEX,
                            (((uint32_t)(r) << 8) |
                             ((uint32_t)(g) << 16) |
                             (uint32_t)(b))
                                << 8u);
    }

    void LightShowRunner::PutPixel(Color color)
    {
        pio_sm_put_blocking(WS_PIO_INSTANCE, WS_STATE_MACHINE_INDEX,
                            (((uint32_t)(color.Red) << 8) |
                             ((uint32_t)(color.Green) << 16) |
                             (uint32_t)(color.Blue))
                                << 8u);
    }

} // namespace PicoLightShow
