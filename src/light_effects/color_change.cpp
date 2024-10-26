#include "pico/stdlib.h"
#include "string.h"

#include "include/types.h"
#include "include/helpers/string_helper.h"

#include "include/light_effects/color_change.h"

namespace PicoLightShow
{
    ColorChange::ColorChange()
    {
    }

    ColorChange::~ColorChange()
    {
    }

    void ColorChange::Draw()
    {
        for (int i = 0; i < ledCount; i++)
        {
            PutPixel(Red,Green,Blue);
        }
    }

    void ColorChange::MoveTimeFrame()
    {
        time++;

        Red += DeltaRed;
        Green += DeltaGreen;
        Blue += DeltaBlue;

        if (time >= TimeFrameSize)
        {
            time = 0;
            colorIndex = colorIndex < Colors.size() - 1 ? colorIndex + 1 : 0;
            CalculateDeltaColor();
        }
    }

    void ColorChange::Init()
    {
        time = 0;
        colorIndex = 0;
        CalculateDeltaColor();
    }

    void ColorChange::CalculateDeltaColor()
    {
        Color c1 = Colors[colorIndex];
        Color c2 = Colors[colorIndex < Colors.size() - 1 ? colorIndex + 1 : 0];

        DeltaRed = (float)(c2.Red - c1.Red)/TimeFrameSize;
        DeltaGreen = (float)(c2.Green - c1.Green)/TimeFrameSize;
        DeltaBlue = (float)(c2.Blue - c1.Blue)/TimeFrameSize;

        Red = c1.Red;
        Green = c1.Green;
        Blue = c1.Blue;
    }

    void ColorChange::SetProperty(char* name, char* value)
    {
        if (strcmp(name, "colors") == 0)
        {
            std::vector<std::string> colors = StringHelper::Split(value, ',');
            Colors.clear();
            for (int i = 0; i < colors.size(); i++)
            {
                Colors.push_back(Color(StringHelper::HexStringToUint32(colors[i])));
            }
            
            Init();
        }

        Draw();
        sleep_ms(5);
    }

    std::string ColorChange::GetConfigurationString()
    {
        return "colors="+StringHelper::ColorsToString(Colors);
    }

} // namespace PicoLightShow
