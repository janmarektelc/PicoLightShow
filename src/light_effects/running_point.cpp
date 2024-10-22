#include "include/light_effects/running_point.h"

namespace PicoLightShow
{

    RunningPoint::RunningPoint()
    {
    }

    RunningPoint::~RunningPoint()
    {
    }

    void RunningPoint::Draw()
    {
        int x = time < ledCount ? time : ledCount + (ledCount - time - 2);

        for (int i = 0; i < ledCount; i++)
        {
            if (i == x)
                PutPixel(255, 0, 0);
            else
                PutPixel(0, 0, 0);
        }
    }

    void RunningPoint::MoveTimeFrame()
    {
        time++;
        if (time / 2 >= ledCount - 1)
        {
            time = 0;
        }
    }

    void RunningPoint::Init()
    {
    }

    void RunningPoint::SetProperty(char* name, char* value)
    {
        
    }

} // namespace PicoLightShow
