#include <stdio.h>

#include "pico/stdlib.h"
#include "hardware/pio.h"
#include "ws2812.pio.h"

#include "include/light_effects/running_point.h"
#include "include/light_effects/jans_ping_pong.h"
#include "include/light_effects/custom_pattern.h"
#include "include/light_effects/color_change.h"
#include "include/persistent_settings/persistent_settings.h"

#include "include/light_show_runner/light_show_runner.h"

namespace PicoLightShow
{
    LighShowEffectDescriptor LightShowRunner::LighShowEffectDescriptors[] = {
        {"Jan's ping pong", "", CreateJansPingPong},
        {"Running point", "custom_pattern_setup.shtml", CreateRunningPoint},
        {"Running colors", "custom_pattern_setup.shtml", CreateRunningColors},
        {"Running colors 1", "custom_pattern_setup.shtml", CreateRunningColors1},
        {"Snakes", "custom_pattern_setup.shtml", CreateSnakes},
        {"Color change", "", CreateColorChange},
    };

    LightEffectBase *LightShowRunner::currentLightEffect = nullptr;

    LightEffectBase *LightShowRunner::CreateRunningPoint()
    {
        CustomPattern *customPattern = new CustomPattern();
        customPattern->ColorPattern = { Color(255,0,0) };
        customPattern->DrawKind = PatternDrawKind::Once;

        return customPattern;
    }

    LightEffectBase *LightShowRunner::CreateJansPingPong()
    {
        return new JansPingPong();
    }

    LightEffectBase *LightShowRunner::CreateRunningColors()
    {
        CustomPattern *customPattern = new CustomPattern();
        customPattern->ColorPattern = {Color(255, 0, 0), Color(255, 255, 0), Color(0, 255, 0), Color(0, 255, 255), Color(0, 0, 255), Color(255, 0, 255)};
        customPattern->DrawKind = PatternDrawKind::Stretch;

        return customPattern;
    }

    LightEffectBase *LightShowRunner::CreateRunningColors1()
    {
        CustomPattern *customPattern = new CustomPattern();
        customPattern->ColorPattern = {Color(255, 0, 0), Color(255, 255, 0), Color(0, 255, 0), Color(0, 255, 255), Color(0, 0, 255), Color(255, 0, 255)};
        customPattern->DrawKind = PatternDrawKind::Repeat;
        customPattern->PingPong = false;
        customPattern->Direction = 1;

        return customPattern;
    }

    LightEffectBase *LightShowRunner::CreateSnakes()
    {
        CustomPattern *customPattern = new CustomPattern();
        customPattern->ColorPattern = {Color(255, 0, 0), Color(0, 0, 0), Color(0, 255, 0), Color(0, 0, 0), Color(0, 0, 255), Color(0, 0, 0)};
        customPattern->DrawKind = PatternDrawKind::Stretch;
        customPattern->PingPong = false;

        return customPattern;
    }

    LightEffectBase *LightShowRunner::CreateColorChange()
    {
        ColorChange *colorChange = new ColorChange();
        //colorChange->Colors = {Color(0,10,0), Color(0,255,0), Color(255,0,0), Color(0,10,0)};
        colorChange->Colors = {Color(0,100,0), Color(0,255,0), Color(0,100,0)};

        return colorChange;
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
            sleep_ms(MINIMAL_REDRAW_DELAY);
        }
    }

    void LightShowRunner::Pool()
    {
        if (PersistentSettings::Settings.IsRunning)
        {          
            currentLightEffect->MoveTimeFrame(); 
            currentLightEffect->Draw();
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
            sleep_ms(MINIMAL_REDRAW_DELAY);
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
            sleep_ms(MINIMAL_REDRAW_DELAY);
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

    void LightShowRunner::SetEffectProperty(char* name, char* value)
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

} // namespace PicoLightShow
