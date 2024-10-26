#include "include/types.h"

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
        Red += DeltaRed;
        Green += DeltaGreen;
        Blue += DeltaBlue;
        for (int i = 0; i < ledCount; i++)
        {
            PutPixel(Red,Green,Blue);
        }
    }

    void ColorChange::MoveTimeFrame()
    {
        time++;
        if (time > TimeFrameSize)
        {
            time = 0;
            colorIndex = colorIndex < Colors.size() - 1 ? colorIndex + 1 : 0;
            CalculateDeltaColor();
        }
    }

    void ColorChange::Init()
    {
        CalculateDeltaColor();
    }

    void ColorChange::CalculateDeltaColor()
    {
        Color c1 = Colors[colorIndex];
        Color c2 = Colors[colorIndex < Colors.size() - 1 ? colorIndex + 1 : 0];

        DeltaRed = (float)(c2.Red - c1.Red)/TimeFrameSize;
        DeltaGreen = (float)(c2.Green - c1.Green)/TimeFrameSize;
        DeltaBlue = (float)(c2.Blue - c1.Blue)/TimeFrameSize;
    }

    void ColorChange::SetProperty(char* name, char* value)
    {
        
    }

    std::string ColorChange::GetConfigurationString()
    {
        return "";
    }

} // namespace PicoLightShow
