#include <string>
#include <vector>
#include <sstream>

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
}