#include "pico/stdlib.h"
#include "string.h"

#include "include/helpers/string_helper.h"
#include "include/light_effects/custom_pattern.h"

namespace PicoLightShow
{
    CustomPattern::CustomPattern()
    {
        Direction = -1;
        PingPong = true;
        DrawKind = PatternDrawKind::Once;
    }

    CustomPattern::~CustomPattern()
    {
    }

    void CustomPattern::Draw()
    {
        if (DrawKind == PatternDrawKind::Once)
            DrawOnce();
        else if (DrawKind == PatternDrawKind::Repeat)
            DrawRepeat();
        else if (DrawKind == PatternDrawKind::Stretch)
            DrawStretch();
    }

    void CustomPattern::MoveTimeFrame()
    {
        MoveTimeFrameStretch();
    }

    void CustomPattern::Init()
    {
    }

    void CustomPattern::DrawRepeat()
    {
        printf("time: %i \n", time);
        for (int i = 0; i < ledCount; i++)
        {
            int patternIndex = (i % ColorPattern.size()) + time;

            if (patternIndex >= ColorPattern.size())
                patternIndex -= ColorPattern.size();

            if (patternIndex < ColorPattern.size())
                PutPixel(ColorPattern.at(patternIndex));
        }
    }

    void CustomPattern::MoveTimeFrameRepeat()
    {
        time += Direction;
        if (time >= (int)ColorPattern.size())
        {
            if (PingPong)
                Direction *= -1;
            else
                time = 0;
        }
        if (time < 0)
        {
            time = 1;
            if (PingPong)
                Direction *= -1;
            else
                time = ColorPattern.size() - 1;
        }
    }

    void CustomPattern::DrawStretch()
    {
        for (int i = 0; i < ledCount; i++)
        {
            // do color pattern movement
            int patternIndex = (float)(i + time) / ((float)ledCount / ColorPattern.size());
            if (patternIndex > ColorPattern.size() - 1)
                patternIndex = patternIndex - (ColorPattern.size());

            if (patternIndex < ColorPattern.size())
                PutPixel(ColorPattern.at(patternIndex));
        }
    }

    void CustomPattern::MoveTimeFrameStretch()
    {
        time += Direction;
        if (time >= ledCount)
        {
            if (PingPong)
                Direction *= -1;
            else
                time = 0;
        }
        if (time < 0)
        {
            time = 1;
            if (PingPong)
                Direction *= -1;
            else
                time = ledCount - 1;
        }
    }

    void CustomPattern::DrawOnce()
    {
        for (int i = 0; i < ledCount; i++)
        {
            int patternIndex = i - time;

            if (patternIndex >= 0 && patternIndex < ColorPattern.size())
                PutPixel(ColorPattern.at(patternIndex));
            else
                PutPixel(0, 0, 0);
        }
    }

    void CustomPattern::MoveTimeFrameOnce()
    {
        time += Direction;
        if (time >= ledCount - (int)ColorPattern.size())
        {
            if (PingPong)
                Direction *= -1;
            else
                time = 0;
        }
        if (time < 0)
        {
            time = 1;
            if (PingPong)
                Direction *= -1;
            else
                time = ledCount - ColorPattern.size() - 1;
        }
    }

    void CustomPattern::SetProperty(char* name, char* value)
    {
        if (strcmp(name, "draw-kind") == 0)
        {
            if (strcmp(value, "0") == 0)
                DrawKind = PatternDrawKind::Once;
            if (strcmp(value, "1") == 0)
                DrawKind = PatternDrawKind::Repeat;
            if (strcmp(value, "2") == 0)
                DrawKind = PatternDrawKind::Stretch;
        }
        if (strcmp(name, "ping-pong") == 0)
        {
            PingPong = strcmp(value, "1") == 0;
        }
        if (strcmp(name, "direction") == 0)
        {
            Direction = strcmp(value, "1") == 0 ? 1 : -1;
        }
        if (strcmp(name, "colors") == 0)
        {
            std::vector<std::string> colors = StringHelper::Split(value, ',');
            ColorPattern.clear();
            for (int i = 0; i < colors.size(); i++)
            {
                ColorPattern.push_back(Color(StringHelper::HexStringToUint32(colors[i])));
            }
        }

        Draw();
        sleep_ms(5);
    }

    std::string CustomPattern::GetConfigurationString()
    {
        return "draw-kind="+std::to_string((int)DrawKind)
            +"&ping-pong="+std::to_string((int)PingPong)
            +"&direction="+(Direction==1?"1":"0")
            +"&colors="+StringHelper::ColorsToString(ColorPattern)
        ;
    }

} // namespace PicoLightShow