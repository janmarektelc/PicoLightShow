#pragma once
#include <string>
#include <map>
#include "data_source.h"

struct HttpRequest {
    std::string method; // GET, POST...
    std::string uri;
    std::string body;
    std::string query;
    std::map<std::string, std::string> parameters;
};

struct HttpResponse {
    int statusCode; // 200, 404, 500...
    std::unique_ptr<IDataSource> data = nullptr;
    std::string contentType = "application/json";

    // helper function to create a response from a string
    static HttpResponse fromJsonString(std::string data, std::string mime = "application/json", int code = 200) {
        HttpResponse res;
        res.statusCode = code;
        res.contentType = mime;
        res.data = std::make_unique<RamSource>(std::move(data));
        return res;
    }
};