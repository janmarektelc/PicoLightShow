#include <stdio.h>

#include "pico/stdlib.h"
#include "hardware/pio.h"
#include "ws2812.pio.h"

#include "include/light_effects/running_point.h"
#include "include/light_effects/custom_pattern.h"
#include "include/persistent_settings/persistent_settings.h"

#include "include/light_show_runner/light_show_runner.h"

namespace PicoLightShow
{
    LighShowEffectDescriptor LightShowRunner::LighShowEffectDescriptors[] = {
        {"RunningPoint", "running_point_setup.shtml", CreateRunningPoint},
        {"CustomPattern", "custom_pattern_setup.shtml", CreateCustomPattern},
        {"CustomPattern1", "custom_pattern_setup.shtml", CreateCustomPattern1},
        {"CustomPattern2", "custom_pattern_setup.shtml", CreateCustomPattern2},
    };

    LightEffectBase *LightShowRunner::currentLightEffect = nullptr;

    LightEffectBase *LightShowRunner::CreateRunningPoint()
    {
        return new RunningPoint();
    }

    LightEffectBase *LightShowRunner::CreateCustomPattern()
    {
        CustomPattern *customPattern = new CustomPattern();
        customPattern->ColorPattern = {Color(255, 0, 0), Color(255, 255, 0), Color(0, 255, 0), Color(0, 255, 255), Color(0, 0, 255), Color(255, 0, 255)};
        customPattern->DrawKind = PatternDrawKind::Stretch;

        return customPattern;
    }

    LightEffectBase *LightShowRunner::CreateCustomPattern1()
    {
        CustomPattern *customPattern = new CustomPattern();
        customPattern->ColorPattern = { Color(0,255,0), Color(255,0,0), Color(255,0,0), Color(0,255,0)};
        customPattern->DrawKind = PatternDrawKind::Once;

        return customPattern;
    }

    LightEffectBase *LightShowRunner::CreateCustomPattern2()
    {
        CustomPattern *customPattern = new CustomPattern();
        customPattern->ColorPattern = {Color(255, 0, 0), Color(255, 255, 0), Color(0, 255, 0), Color(0, 255, 255), Color(0, 0, 255), Color(255, 0, 255)};
        customPattern->DrawKind = PatternDrawKind::Repeat;

        return customPattern;
    }

    void LightShowRunner::Init()
    {
        uint offset = pio_add_program(WS_PIO_INSTANCE, &ws2812_program);
        ws2812_program_init(WS_PIO_INSTANCE, WS_STATE_MACHINE_INDEX, offset, WS2812_PIN, 800000, IS_RGBW);

        currentLightEffect = LighShowEffectDescriptors[PersistentSettings::Settings.EffectIndex].GetInstance();
        currentLightEffect->SetLedCount(PersistentSettings::Settings.LedCount);
        currentLightEffect->SetBrightness(PersistentSettings::Settings.Brightness);

        currentLightEffect->Init();
        
        if (!PersistentSettings::Settings.IsRunning)
        {
            currentLightEffect->Draw();
            sleep_ms(5);
        }
    }

    void LightShowRunner::Pool()
    {
        if (PersistentSettings::Settings.IsRunning)
        {
            currentLightEffect->Draw();
            currentLightEffect->MoveTimeFrame();
        }
        sleep_ms(PersistentSettings::Settings.Delay);
    }

    void LightShowRunner::Start()
    {
        PersistentSettings::Settings.IsRunning = true;
    }

    void LightShowRunner::Stop()
    {
        PersistentSettings::Settings.IsRunning = false;
    }

    bool LightShowRunner::GetIsRunning()
    {
        return PersistentSettings::Settings.IsRunning;
    }

    uint32_t LightShowRunner::GetFrameDelay()
    {
        return PersistentSettings::Settings.Delay;
    }

    void LightShowRunner::SetFrameDelay(uint32_t frameDelay)
    {
        if (frameDelay >= 1 && frameDelay <= 10000)
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
        currentLightEffect->SetLedCount(ledCount);
    }

    uint8_t LightShowRunner::GetBrightness()
    {
        return PersistentSettings::Settings.Brightness;
    }

    void LightShowRunner::SetBrightness(uint8_t brightness)
    {
        PersistentSettings::Settings.Brightness = brightness;
        currentLightEffect->SetBrightness(brightness);
        if (!PersistentSettings::Settings.IsRunning)
        {
            currentLightEffect->Draw();
            sleep_ms(5);
        }
    }

    uint32_t LightShowRunner::GetEffect()
    {
        return PersistentSettings::Settings.EffectIndex;
    }

    void LightShowRunner::SetEffect(uint32_t effect)
    {
        PersistentSettings::Settings.EffectIndex = effect;
        delete currentLightEffect;

        currentLightEffect = LighShowEffectDescriptors[effect].GetInstance();
        currentLightEffect->SetLedCount(PersistentSettings::Settings.LedCount);
        currentLightEffect->SetBrightness(PersistentSettings::Settings.Brightness);

        currentLightEffect->Init();

        if (!PersistentSettings::Settings.IsRunning)
        {
            currentLightEffect->Draw();
            sleep_ms(5);
        }
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

} // namespace PicoLightShow
