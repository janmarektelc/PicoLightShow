#pragma once
#include <cstdint>
#include <cstddef>
#include <string>
#include <memory>
#include <algorithm>
#include <cstring>

class IDataSource {
public:
    virtual ~IDataSource() = default;
    virtual size_t read(uint8_t* buffer, size_t size, size_t offset) = 0;
    virtual size_t size() = 0;
};

// implementation for flash memory (static assets)
class FlashSource : public IDataSource {
    const uint8_t* data;
    size_t sz;
public:
    FlashSource(const uint8_t* d, size_t s) : data(d), sz(s) {}
    size_t read(uint8_t* buf, size_t len, size_t off) override {
        size_t toRead = std::min(len, sz - off);
        std::memcpy(buf, data + off, toRead);
        return toRead;
    }
    size_t size() override { return sz; }
};

// implementation for RAM (dynamic strings)
class RamSource : public IDataSource {
    std::string data;
public:
    RamSource(std::string d) : data(std::move(d)) {}
    size_t read(uint8_t* buf, size_t len, size_t off) override {
        size_t toRead = std::min(len, data.size() - off);
        std::memcpy(buf, data.data() + off, toRead);
        return toRead;
    }
    size_t size() override { return data.size(); }
};