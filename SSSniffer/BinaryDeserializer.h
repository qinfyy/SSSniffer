#pragma once
#include <string>
#include <vector>
#include <nlohmann/json.hpp>

struct Packet {
    int64_t time;
    bool fromServer;
    int32_t packetId;
    std::string packetName;
    nlohmann::json object;
    std::string raw;
};

bool LoadPbFromMemory(const void* data, int len);

bool LoadPbFromFile(const std::string& path);

bool DeserializeToJson(int32_t msgId, const void* binaryData, int dataLen, std::string& outJson);

const std::string& GetMsgName(int32_t msgId);

bool DeserializeToPacketList(int32_t msgId, const void* binaryData, int dataLen, bool fromServer, std::vector<Packet>& outPackets);