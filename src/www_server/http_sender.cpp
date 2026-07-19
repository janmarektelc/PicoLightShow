#include "include/www_server/http_sender.h"
#include <algorithm>
#include <include/www_server/models.h>
#include <string>
#include <pico/cyw43_arch.h>


void HttpSender::send(tcp_pcb* tpcb, HttpResponse& res) {
    auto* s = new HttpSender();
    
    // get the data source from the response, or create an empty one if none is provided
    if (res.data) {
        s->source = std::move(res.data);
    } else {
        s->source = std::make_unique<RamSource>("");
    }

    std::string statusText = (res.statusCode == 200) ? "200 OK" : 
                              (res.statusCode == 404) ? "404 Not Found" : "500 Internal Error";
    
    std::string header = "HTTP/1.1 " + statusText + "\r\n" +
                         "Content-Type: " + res.contentType + "\r\n" +
                         "Content-Length: " + std::to_string(s->source->size()) + "\r\n" +
                         "Connection: close\r\n\r\n";
    
    tcp_arg(tpcb, s);
    
    // TCP callback when data is acknowledged by the client
    tcp_sent(tpcb, [](void* arg, tcp_pcb* tpcb, u16_t) -> err_t {
        static_cast<HttpSender*>(arg)->transmit(tpcb);
        return ERR_OK;
    });

    // Error callback to clean up the HttpSender instance if the connection is closed unexpectedly
    tcp_err(tpcb, [](void* arg, err_t) { 
        delete static_cast<HttpSender*>(arg); 
    });

    // send header and start transmitting the body
    tcp_write(tpcb, header.c_str(), header.length(), TCP_WRITE_FLAG_COPY);
    s->transmit(tpcb);
}

void HttpSender::transmit(tcp_pcb* tpcb) {
    if (!tpcb) {
        delete this;
        return;
    }

    // While the TCP stack has free space in its send buffer,
    // we keep filling data in safe 1024-byte blocks to prevent RAM and CPU starvation
    while (true) {
        size_t free_space = tcp_sndbuf(tpcb);
        if (free_space == 0) {
            break; // Buffer is full, wait for ACK from the client
        }

        uint8_t buffer[1024];
        size_t to_read = std::min(sizeof(buffer), free_space);
        size_t read = source->read(buffer, to_read, offset);
        
        if (read > 0) {
            // Write data using TCP_WRITE_FLAG_COPY with a manageable 1024B chunk
            err_t err = tcp_write(tpcb, buffer, read, TCP_WRITE_FLAG_COPY);
            
            if (err == ERR_OK) {
                offset += read;
                tcp_output(tpcb); // Push the data out immediately
            } 
            else if (err == ERR_MEM) {
                break; // LwIP out of pbuf memory, exit and wait for the next cycle
            } 
            else {
                tcp_arg(tpcb, nullptr);
                tcp_close(tpcb);
                delete this;
                return;
            }
        } else {
            // File fully read, check if the transmission queue is completely empty
            u8_t queued_lens = tcp_sndqueuelen(tpcb);
            if (queued_lens == 0) {
                tcp_arg(tpcb, nullptr);
                tcp_close(tpcb);
                delete this;
            }
            break;
        }
    }
}

