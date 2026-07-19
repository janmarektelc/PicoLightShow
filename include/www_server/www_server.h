#pragma once
#include "lwip/tcp.h"
#include <string>
#include "models.h"

class WwwServer {
public:
    virtual ~WwwServer() = default;
    virtual HttpResponse onRequest(const HttpRequest& req) = 0;
    void start(uint16_t port = 80);
private:
    // LwIp callbacks
    static err_t s_accept_callback(void* arg, tcp_pcb* newpcb, err_t err);
    static err_t s_recv_callback(void* arg, tcp_pcb* tpcb, pbuf* p, err_t err);

    HttpRequest parseRequest(const std::string& reqStr);
    void parseQueryString(const std::string& queryStr, std::map<std::string, std::string>& params);
};
