#ifndef PICO_LIGHT_SHOW_DDP_H_
#define PICO_LIGHT_SHOW_DDP_H_

#include <stdint.h>

namespace PicoLightShow
{
    class DDP
    {
    private:
        static uint8_t* led_buffer;
        static uint32_t current_buf_size;
        static uint16_t num_leds;
        static uint32_t last_packet_time; 
        static bool is_streaming;

    public:
        DDP() = delete;
        static bool Init(uint16_t initial_num_leds);
        static bool Resize(uint16_t new_num_leds);
        static void Deinit();
        static void Poll();
        static bool IsActive();
        static void ResetTimeout();
        
        static uint8_t* GetBuffer(uint32_t* out_size) {
            if (out_size) *out_size = current_buf_size;
            return led_buffer;
        }
    };
}

#endif // PICO_LIGHT_SHOW_DDP_H_
