#include "include/www_server/www_server.h"
#include "include/www_server/http_sender.h"

void WwwServer::start(uint16_t port) {
    tcp_pcb* pcb = tcp_new();
    if (!pcb) return;

    if (tcp_bind(pcb, IP_ADDR_ANY, port) != ERR_OK) {
        tcp_close(pcb);
        return;
    }

    pcb = tcp_listen(pcb);
    if (!pcb) return;

    tcp_arg(pcb, this);
    tcp_accept(pcb, s_accept_callback);
}

err_t WwwServer::s_accept_callback(void* arg, tcp_pcb* newpcb, err_t err) {
    auto* server = static_cast<WwwServer*>(arg);
    
    tcp_arg(newpcb, server);
    tcp_recv(newpcb, s_recv_callback);
    
    return ERR_OK;
}

err_t WwwServer::s_recv_callback(void* arg, tcp_pcb* tpcb, pbuf* p, err_t err) {
    auto* server = static_cast<WwwServer*>(arg);

    if (!p) {
        tcp_close(tpcb);
        return ERR_OK;
    }

    std::string reqStr(static_cast<char*>(p->payload), p->tot_len);

    HttpRequest req = server->parseRequest(reqStr);

    HttpResponse res = server->onRequest(req);
    
    HttpSender::send(tpcb, res);
    
    tcp_recved(tpcb, p->tot_len);
    pbuf_free(p);

    return ERR_OK;
}

HttpRequest WwwServer::parseRequest(const std::string& reqStr) {
    HttpRequest req;
    
    // parse method like GET, POST, etc.
    size_t mEnd = reqStr.find(' ');
    if (mEnd != std::string::npos) {
        req.method = reqStr.substr(0, mEnd);
    }

    // parse path and query string
    size_t pEnd = reqStr.find(' ', mEnd + 1);
    if (mEnd != std::string::npos && pEnd != std::string::npos) {
        std::string fullPath = reqStr.substr(mEnd + 1, pEnd - (mEnd + 1));

        size_t qPos = fullPath.find('?');
        if (qPos == std::string::npos) {
            req.uri = fullPath;
        } else {
            req.uri = fullPath.substr(0, qPos);
            req.query = fullPath.substr(qPos + 1);
            parseQueryString(req.query, req.parameters);
        }
    }

    // parse body
    size_t bStart = reqStr.find("\r\n\r\n");
    if (bStart != std::string::npos) {
        req.body = reqStr.substr(bStart + 4);
    }

    return req;
}

void WwwServer::parseQueryString(const std::string& queryStr, std::map<std::string, std::string>& params) {
    size_t start = 0;
    size_t end = 0;

    while ((end = queryStr.find('&', start)) != std::string::npos) {
        std::string pair = queryStr.substr(start, end - start);
        size_t eqPos = pair.find('=');
        if (eqPos != std::string::npos) {
            params[pair.substr(0, eqPos)] = pair.substr(eqPos + 1);
        }
        start = end + 1;
    }

    std::string pair = queryStr.substr(start);
    size_t eqPos = pair.find('=');
    if (eqPos != std::string::npos) {
        params[pair.substr(0, eqPos)] = pair.substr(eqPos + 1);
    }
}