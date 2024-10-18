#include "lwip/apps/httpd.h"
#include "pico/cyw43_arch.h"

#include "lwipopts.h"
#include "include/light_show_runner/light_show_runner.h"
#include "include/persistent_settings/persistent_settings.h"
#include "include/www_server/cgi.h"
#include "include/www_server/ssi.h"


namespace PicoLightShow
{

    // Max length of the tags defaults to be 8 chars
    // LWIP_HTTPD_MAX_TAG_NAME_LEN
    const char *__not_in_flash("httpd") ssi_example_tags[] = {
        "running",   // 0
        "delay", // 1
        "bright", // 2
        "efflst", //3
        "ledcnt", //4
        "wmodes", //5
        "wifissid", //6
        "ipauto", //7
        "ip1", //8
        "ip2", //9
        "ip3", //10
        "ip4", //11
        "netmask1", //12
        "netmask2", //13
        "netmask3", //14
        "netmask4", //15
        "gw1", //16
        "gw2", //17
        "gw3", //18
        "gw4", //19
    };

    u16_t __time_critical_func(ssi_handler)(int iIndex, char *pcInsert, int iInsertLen)
    {
        size_t printed;
        switch (iIndex)
        {
        case 0: /* "running" */
            if (LightShowRunner::GetIsRunning())
                printed = snprintf(pcInsert, iInsertLen, "checked");
            else
                printed = sniprintf(pcInsert, iInsertLen, "");
            break;
        case 1: /* "delay" */
        {
            printed = snprintf(pcInsert, iInsertLen, "%i", LightShowRunner::GetFrameDelay());
            break;
        }
        case 2: /* "bright" */
        {
            printed = snprintf(pcInsert, iInsertLen, "%i", LightShowRunner::GetBrightness());
            break;
        }
        case 3: /* "efflst"*/
        {
            std::vector<std::string> effectNames = LightShowRunner::GetEffectNames();
            int currentEffect = LightShowRunner::GetEffect();
            std::string effectsList = "";
            for (int i=0; i<effectNames.size(); i++)
            {
                effectsList += "<option value=\""+std::to_string(i)+"\""; 
                if (i==currentEffect)
                    effectsList += " selected=\"selected\"";
                effectsList += ">"+effectNames[i]+"</option>";
            }
            printed = snprintf(pcInsert, iInsertLen, "%s", effectsList.c_str());
            break;
        }
        case 4: /* "ledcnt" */
        {
            printed = snprintf(pcInsert, iInsertLen, "%i", LightShowRunner::GetLedCount());
            break;
        }
        case 5: /* "wmodes" */
        {
            std::string wmodes = "";
            if (PersistentSettings::Settings.WifiMode == AP)
            {
                printed = snprintf(pcInsert, iInsertLen, "%s", "<option value=\"0\" selected=\"selected\">Access point</option><option value=\"1\">Client</option>");
            }
            else
            {
                printed = snprintf(pcInsert, iInsertLen, "%s", "<option value=\"0\">Access point</option><option value=\"1\" selected=\"selected\">Client</option>");
            }
            break;
        }
        case 6: /* "wifissid" */
        {
            printed = snprintf(pcInsert, iInsertLen, "%s", PersistentSettings::Settings.WifiName);
            break;
        }
        case 7: /* "ipauto" */
        {
            if (PersistentSettings::Settings.IsDhcp)
                printed = snprintf(pcInsert, iInsertLen, "checked");
            else
                printed = sniprintf(pcInsert, iInsertLen, "");
            break;
        }
        case 8: /* "ip1" */
        {
            printed = snprintf(pcInsert, iInsertLen, "%i", PersistentSettings::Settings.DeviceAddress & 0xFF);
            break;
        }
        case 9: /* "ip2" */
        {
            printed = snprintf(pcInsert, iInsertLen, "%i", (PersistentSettings::Settings.DeviceAddress >> 8) & 0xFF);
            break;
        }
        case 10: /* "ip3" */
        {
            printed = snprintf(pcInsert, iInsertLen, "%i", (PersistentSettings::Settings.DeviceAddress >> 16) & 0xFF);
            break;
        }
        case 11: /* "ip4" */
        {
            printed = snprintf(pcInsert, iInsertLen, "%i", (PersistentSettings::Settings.DeviceAddress >> 24) & 0xFF);
            break;
        }
        case 12: /* "netmask1" */
        {
            printed = snprintf(pcInsert, iInsertLen, "%i", PersistentSettings::Settings.NetMask & 0xFF);
            break;
        }
        case 13: /* "netmask2" */
        {
            printed = snprintf(pcInsert, iInsertLen, "%i", (PersistentSettings::Settings.NetMask >> 8) & 0xFF);
            break;
        }
        case 14: /* "netmask3" */
        {
            printed = snprintf(pcInsert, iInsertLen, "%i", (PersistentSettings::Settings.NetMask >> 16) & 0xFF);
            break;
        }
        case 15: /* "netmask4" */
        {
            printed = snprintf(pcInsert, iInsertLen, "%i", (PersistentSettings::Settings.NetMask >> 24) & 0xFF);
            break;
        }
        case 16: /* "gw1" */
        {
            printed = snprintf(pcInsert, iInsertLen, "%i", PersistentSettings::Settings.GatewayAddress & 0xFF);
            break;
        }
        case 17: /* "gw2" */
        {
            printed = snprintf(pcInsert, iInsertLen, "%i", (PersistentSettings::Settings.GatewayAddress >> 8) & 0xFF);
            break;
        }
        case 18: /* "gw3" */
        {
            printed = snprintf(pcInsert, iInsertLen, "%i", (PersistentSettings::Settings.GatewayAddress >> 16) & 0xFF);
            break;
        }
        case 19: /* "gw4" */
        {
            printed = snprintf(pcInsert, iInsertLen, "%i", (PersistentSettings::Settings.GatewayAddress >> 24) & 0xFF);
            break;
        }
        break;
        default: /* unknown tag */
            printed = 0;
            break;
        }
        LWIP_ASSERT("sane length", printed <= 0xFFFF);
        return (u16_t)printed;
    }

    void ssi_init()
    {
        for (size_t i = 0; i < LWIP_ARRAYSIZE(ssi_example_tags); i++)
        {
            LWIP_ASSERT("tag too long for LWIP_HTTPD_MAX_TAG_NAME_LEN",
                        strlen(ssi_example_tags[i]) <= LWIP_HTTPD_MAX_TAG_NAME_LEN);
        }

        http_set_ssi_handler(ssi_handler, ssi_example_tags, LWIP_ARRAYSIZE(ssi_example_tags));
    }

} // namespace PicoLightShow
