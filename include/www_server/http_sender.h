#pragma once
#include "lwip/tcp.h"
#include "models.h"

class HttpSender {
    std::unique_ptr<IDataSource> source;
    size_t offset = 0;
public:
    static void send(tcp_pcb* tpcb, HttpResponse& res);
    void transmit(tcp_pcb* tpcb);
};