#include "include/ddp/ddp.h"
#include <stdio.h>
#include <string.h>
#include "pico/stdlib.h"
#include "hardware/pio.h"
#include "pico/cyw43_arch.h"
#include "lwip/udp.h"
#include "lwip/pbuf.h"
#include <include/constants.h>

namespace PicoLightShow
{
    static struct udp_pcb* ddp_pcb = nullptr;
    
    uint8_t* DDP::led_buffer = nullptr;
    uint32_t DDP::current_buf_size = 0;
    uint16_t DDP::num_leds = 0;

    uint32_t DDP::last_packet_time = 0;
    bool DDP::is_streaming = false;

    static bool new_data_available = false;

    struct __attribute__((__packed__)) DDPHeader {
        uint8_t flags;
        uint8_t type;
        uint8_t id;
        uint8_t count;
        uint32_t offset;
        uint16_t len;
    };

    static void ddp_recv_callback(void *arg, struct udp_pcb *pcb, struct pbuf *p, const ip_addr_t *addr, u16_t port) {
        uint32_t buf_size = 0;
        uint8_t* buffer = DDP::GetBuffer(&buf_size);

        if (p == nullptr || buffer == nullptr) {
            if (p != nullptr) pbuf_free(p);
            return;
        }

        if (p->tot_len >= sizeof(DDPHeader)) {
            DDPHeader header;
            pbuf_copy_partial(p, &header, sizeof(DDPHeader), 0);

            uint32_t byte_offset = __builtin_bswap32(header.offset); 
            uint16_t incoming_data_len = __builtin_bswap16(header.len); 

            if (byte_offset < buf_size) {
                uint32_t bytes_to_copy = incoming_data_len;

                if (byte_offset + bytes_to_copy > buf_size) {
                    bytes_to_copy = buf_size - byte_offset;
                }

                pbuf_copy_partial(p, buffer + byte_offset, bytes_to_copy, sizeof(DDPHeader));

                uint32_t bytes_written_total = byte_offset + bytes_to_copy;
                if (bytes_written_total < buf_size) {
                    memset(buffer + bytes_written_total, 0, buf_size - bytes_written_total);
                }
                DDP::ResetTimeout();
                new_data_available = true;
            }            
        }
    
        pbuf_free(p);
    }

    bool DDP::Init(uint16_t initial_num_leds) {
        if (!Resize(initial_num_leds)) {
                return false;
            }

            if (ddp_pcb == nullptr) {
                ddp_pcb = udp_new();
                if (ddp_pcb != nullptr) {
                    err_t err = udp_bind(ddp_pcb, IP_ADDR_ANY, DDP_PORT);
                    if (err == ERR_OK) {
                        udp_recv(ddp_pcb, ddp_recv_callback, nullptr);
                    } else {
                        udp_remove(ddp_pcb);
                        ddp_pcb = nullptr;
                        return false;
                    }
                } else {
                    return false;
                }
            }

            return true;
    }

    void DDP::Deinit() {
        if (led_buffer != nullptr) {
            delete[] led_buffer;
            led_buffer = nullptr;
        }
        current_buf_size = 0;
        num_leds = 0;
    }

    bool DDP::Resize(uint16_t new_num_leds) {
        if (new_num_leds == 0) return false;
        
        uint32_t needed_size = new_num_leds * 3;

        if (needed_size == current_buf_size && led_buffer != nullptr) {
            num_leds = new_num_leds;
            return true; 
        }

        new_data_available = false;

        if (led_buffer != nullptr) {
            free(led_buffer); 
            led_buffer = nullptr;
        }

        led_buffer = (uint8_t*)malloc(needed_size);
        
        if (led_buffer == nullptr) {
            current_buf_size = 0;
            num_leds = 0;
            return false; 
        }

        memset(led_buffer, 0, needed_size);
        current_buf_size = needed_size;
        num_leds = new_num_leds;

        return true;
    }   

    void DDP::Poll() {
        if (new_data_available) {
            new_data_available = false;

            for (int i = 0; i < num_leds; i++) {
                uint8_t r = led_buffer[i * 3 + 0];
                uint8_t g = led_buffer[i * 3 + 1];
                uint8_t b = led_buffer[i * 3 + 2];

                uint32_t pio_pixel_data = ((r << 8) |
                                          (g << 16) |
                                          (b)) << 8u;

                pio_sm_put_blocking(WS_PIO_INSTANCE, WS_STATE_MACHINE_INDEX, pio_pixel_data);
            }
        }
    }

    void DDP::ResetTimeout() {
        last_packet_time = to_ms_since_boot(get_absolute_time()); // Pico SDK funkce pro milisekundy
        is_streaming = true;
    }

    bool DDP::IsActive() {
        if (!is_streaming) {
            return false;
        }

        uint32_t current_time = to_ms_since_boot(get_absolute_time());
        
        if (current_time - last_packet_time > DDP_TIMEOUT_MS) {
            is_streaming = false;
            return false;
        }

        return true;
    }
}