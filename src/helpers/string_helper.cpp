#include <sstream>
#include <iostream>
#include <iomanip>

#include "include/helpers/string_helper.h"

namespace PicoLightShow
{
    std::vector<std::string> StringHelper::Split(std::string str, char delim)
    {
        std::stringstream input(str);
        std::string segment;
        std::vector<std::string> segments;

        while (std::getline(input, segment, delim))
        {
            segments.push_back(segment);
        }

        return segments;
    }

    std::string StringHelper::ColorsToString(std::vector<Color> colors)
    {
        std::stringstream ss;
        for (int i = 0; i < colors.size(); i++)
        {
            if (i > 0)
                ss << ",";

            ss << std::setw(2) << std::setfill('0') << std::hex << (int)colors[i].Red;
            ss << std::setw(2) << std::setfill('0') << std::hex << (int)colors[i].Green;
            ss << std::setw(2) << std::setfill('0') << std::hex << (int)colors[i].Blue;
        }

        return ss.str();
    }

    std::string StringHelper::ColorToString(Color color)
    {
        std::stringstream ss;
        ss << std::setw(2) << std::setfill('0') << std::hex << (int)color.Red;
        ss << std::setw(2) << std::setfill('0') << std::hex << (int)color.Green;
        ss << std::setw(2) << std::setfill('0') << std::hex << (int)color.Blue;
        return ss.str();
    }

    std::string StringHelper::ColorToString(uint8_t r, uint8_t g, uint8_t b)
    {
        std::stringstream ss;
        ss << std::setw(2) << std::setfill('0') << std::hex << (int)r;
        ss << std::setw(2) << std::setfill('0') << std::hex << (int)g;
        ss << std::setw(2) << std::setfill('0') << std::hex << (int)b;
        return ss.str();
    }

    uint32_t StringHelper::HexStringToUint32(std::string hexValue)
    {
        uint32_t outVal;
        std::stringstream ss;
        ss << std::hex << hexValue;
        ss >> outVal;

        return outVal;
    }
}