#ifndef PICO_LIGHT_SHOW_CGI_H_
#define PICO_LIGHT_SHOW_CGI_H_

namespace PicoLightShow
{

    /* Initialize the CGI handler */
    void cgi_init(void);

    const char *cgi_handler_start_effect(int iIndex, int iNumParams, char *pcParam[], char *pcValue[]);
    const char *cgi_handler_stop_effect(int iIndex, int iNumParams, char *pcParam[], char *pcValue[]);
    const char *cgi_handler_switch_effect(int iIndex, int iNumParams, char *pcParam[], char *pcValue[]);
    const char *cgi_handler_configure_runner(int iIndex, int iNumParams, char *pcParam[], char *pcValue[]);
    const char *cgi_handler_reboot(int iIndex, int iNumParams, char *pcParam[], char *pcValue[]);
    const char *cgi_handler_reset_to_default(int iIndex, int iNumParams, char *pcParam[], char *pcValue[]);

} // namespace PicoLightShow

#endif // PICO_LIGHT_SHOW_CGI_H_
