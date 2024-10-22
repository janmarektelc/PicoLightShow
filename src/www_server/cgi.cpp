#include "lwip/apps/httpd.h"
#include "pico/cyw43_arch.h"
#include "hardware/watchdog.h"

#include "lwipopts.h"
#include "include/light_show_runner/light_show_runner.h"
#include "include/persistent_settings/persistent_settings.h"

#include "include/www_server/cgi.h"

namespace PicoLightShow
{

    static const tCGI cgi_handlers[] = {
        {"/start_effect", cgi_handler_start_effect},
        {"/stop_effect", cgi_handler_stop_effect},
        {"/switch_effect", cgi_handler_switch_effect},
        {"/configure_runner", cgi_handler_configure_runner},
        {"/reboot", cgi_handler_reboot},
        {"/reset_to_default", cgi_handler_reset_to_default},
        {"/set_effect_property", cgi_handler_set_effect_property}};

    const char *cgi_handler_start_effect(int iIndex, int iNumParams, char *pcParam[], char *pcValue[])
    {
        LightShowRunner::Start();
        return "/success.html";
    }

    const char *cgi_handler_stop_effect(int iIndex, int iNumParams, char *pcParam[], char *pcValue[])
    {
        LightShowRunner::Stop();
        return "/success.html";
    }

    const char *cgi_handler_configure_runner(int iIndex, int iNumParams, char *pcParam[], char *pcValue[])
    {
        for (int i=0; i<iNumParams; i++)
        {
            if (strcmp(pcParam[i], "speed") == 0)
            {
                LightShowRunner::SetFrameDelay(atoi(pcValue[i]));
            }

            if (strcmp(pcParam[i], "brightness") == 0)
            {
                LightShowRunner::SetBrightness((uint8_t)atoi(pcValue[i]));
            }
            if (strcmp(pcParam[i], "ledCount") == 0)
            {
                LightShowRunner::SetLedCount(atoi(pcValue[i]));
            }
        }
        return "/success.html";
    }

    const char *cgi_handler_switch_effect(int iIndex, int iNumParams, char *pcParam[], char *pcValue[])
    {
        for (int i=0; i<iNumParams; i++)
        {
            if (strcmp(pcParam[i], "effect") == 0)
            {
                LightShowRunner::SetEffect(atoi(pcValue[i]));
            }
        }
        return "/success.html";
    }

    const char *cgi_handler_reboot(int iIndex, int iNumParams, char *pcParam[], char *pcValue[])
    {
        watchdog_reboot(0,0,100);
        return "/success.html";
    }

    const char *cgi_handler_reset_to_default(int iIndex, int iNumParams, char *pcParam[], char *pcValue[])
    {
        PersistentSettings::SetToDefault();
        PersistentSettings::Save();
        watchdog_reboot(0,0,100);
        return "/success.html";
    }

    const char *cgi_handler_set_effect_property(int iIndex, int iNumParams, char *pcParam[], char *pcValue[])
    {
        for (int i=0; i<iNumParams; i++)
        {
            LightShowRunner::SetEffectProperty(pcParam[i], pcValue[i]);
        }
        return "/success.html";
    }

    /* Initialize the CGI handler */
    void cgi_init(void)
    {
        http_set_cgi_handlers(cgi_handlers, 7);
    }

} // namespace PicoLightShow
