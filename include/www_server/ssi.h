#ifndef PICO_LIGHT_SHOW_SSI_H_
#define PICO_LIGHT_SHOW_SSI_H_

namespace PicoLightShow
{

    u16_t __time_critical_func(ssi_handler)(int iIndex, char *pcInsert, int iInsertLen);
    void ssi_init();

} // namespace PicoLightShow

#endif // PICO_LIGHT_SHOW_SSI_H_
