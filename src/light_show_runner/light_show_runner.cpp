#include <stdio.h>

#include "pico/stdlib.h"
#include "hardware/pio.h"
#include "ws2812.pio.h"

#include "include/light_effects/running_point.h"
#include "include/light_effects/jans_ping_pong.h"
#include "include/light_effects/custom_pattern.h"
#include "include/light_effects/color_change.h"
#include "include/persistent_settings/persistent_settings.h"
#include "include/helpers/string_helper.h"

#include "include/light_show_runner/light_show_runner.h"

namespace PicoLightShow
{
    LighShowEffectDescriptor LightShowRunner::LighShowEffectDescriptors[] = {
        {"Running point", "custom_pattern_setup.shtml", CreateCustomPattern, "draw-kind=0&ping-pong=1&direction=1&colors=ff0000"},
        {"Running colors", "custom_pattern_setup.shtml", CreateCustomPattern, "draw-kind=2&ping-pong=1&direction=0&colors=ff0000,ffff00,00ff00,00ffff,0000ff,ff00ff"},
        {"Running colors 1", "custom_pattern_setup.shtml", CreateCustomPattern, "draw-kind=1&ping-pong=0&direction=1&colors=ff0000,ffff00,00ff00,00ffff,0000ff,ff00ff"},
        {"Snakes", "custom_pattern_setup.shtml", CreateCustomPattern, "draw-kind=2&ping-pong=0&direction=0&colors=ff0000,000000,00ff00,000000,0000ff,000000"},
        {"Color change", "change_color_setup.shtml", CreateColorChange, "colors=640000,ff0000,006400,00ff00,000064,0000ff"},
        {"Breath", "change_color_setup.shtml", CreateColorChange, "colors=640000,ff0000"},
        // {"Jan's ping pong", "", CreateJansPingPong, ""},
    };

    LightEffectBase *LightShowRunner::currentLightEffect = nullptr;


    LightEffectBase *LightShowRunner::CreateCustomPattern()
    {
        return new CustomPattern();
    }

    LightEffectBase *LightShowRunner::CreateColorChange()
    {
        return new ColorChange();
    }

    LightEffectBase *LightShowRunner::CreateJansPingPong()
    {
        return new JansPingPong();
    }

    void LightShowRunner::Init()
    {
        uint offset = pio_add_program(WS_PIO_INSTANCE, &ws2812_program);
        ws2812_program_init(WS_PIO_INSTANCE, WS_STATE_MACHINE_INDEX, offset, WS2812_PIN, 800000, IS_RGBW);

        currentLightEffect = LighShowEffectDescriptors[PersistentSettings::Settings.EffectIndex].GetInstance();
        currentLightEffect->SetLedCount(PersistentSettings::Settings.LedCount);
        currentLightEffect->SetBrightness(PersistentSettings::Settings.Brightness);

        SetEffectConfigurationString(LighShowEffectDescriptors[PersistentSettings::Settings.EffectIndex].Parameters);

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
        LighShowEffectDescriptors[PersistentSettings::Settings.EffectIndex].Parameters = currentLightEffect->GetConfigurationString();

        PersistentSettings::Settings.EffectIndex = effect;
        delete currentLightEffect;

        currentLightEffect = LighShowEffectDescriptors[effect].GetInstance();
        currentLightEffect->SetLedCount(PersistentSettings::Settings.LedCount);
        currentLightEffect->SetBrightness(PersistentSettings::Settings.Brightness);

        SetEffectConfigurationString(LighShowEffectDescriptors[effect].Parameters);

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

} // namespace PicoLightShow
