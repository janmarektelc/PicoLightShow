#include "include/light_effects/running_point.h"

namespace PicoLightShow
{

    RunningPoint::RunningPoint()
    {
    }

    RunningPoint::~RunningPoint()
    {
    }

    void RunningPoint::Draw(std::vector<uint32_t>* buffer)
    {
        int x = time < ledCount ? time : ledCount + (ledCount - time - 2);
        std::fill(buffer->begin(), buffer->end(), 0u);
        buffer->at(x) = EncodeColor(255, 0, 0);
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

    void RunningPoint::SetProperty(const char* name, const char* value)
    {
        
    }

    std::string RunningPoint::GetConfigurationString()
    {
        return "";
    }

} // namespace PicoLightShow
