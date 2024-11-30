#include "include/light_effects/jans_ping_pong.h"

namespace PicoLightShow
{

    JansPingPong::JansPingPong()
    {
    }

    JansPingPong::~JansPingPong()
    {
    }

    void JansPingPong::Draw()
    {
        int x = (time < ledCount - 2 ? time : ledCount + (ledCount - time - 6)) + 1;
        for (int i = 0; i < ledCount; i++)
        {
            if (i == 0)
                PutPixel(0,255,0);
            else if (i == ledCount - 1)
                PutPixel(0,255,0);
            else if (i == x)
                PutPixel(255, 0, 0);
            else
                PutPixel(0, 0, 0);
        }
    }

    void JansPingPong::MoveTimeFrame()
    {
        time++;
        if (time >= (ledCount - 3)*2)
        {
            time = 0;
        }
    }

    void JansPingPong::Init()
    {
    }

    void JansPingPong::SetProperty(const char* name, const char* value)
    {
        
    }

    std::string JansPingPong::GetConfigurationString()
    {
        return "";
    }

} // namespace PicoLightShow
