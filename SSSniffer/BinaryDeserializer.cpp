#include "pch.h"
#include "BinaryDeserializer.h"

#include <fstream>
#include <iostream>
#include <memory>
#include <string>
#include <WinSock2.h>

#include <google/protobuf/descriptor.h>
#include <google/protobuf/dynamic_message.h>
#include <google/protobuf/descriptor.pb.h>
#include <google/protobuf/util/json_util.h>

#include "Data.h"
#include "PrintHelper.h"
#include "Util.h"

static google::protobuf::DescriptorPool g_descriptorPool;
static google::protobuf::DynamicMessageFactory g_factory;

bool LoadPbFromMemory(const void* data, int len) {
    google::protobuf::FileDescriptorSet fds;
    if (!fds.ParseFromArray(data, len)) {
        DebugPrintA("unable to parse data from memory\n");
        return false;
    }

    for (int i = 0; i < fds.file_size(); ++i) {
        const google::protobuf::FileDescriptor* fd = g_descriptorPool.BuildFile(fds.file(i));
        if (!fd) {
            DebugPrintA("unable to build descriptor: %s\n", fds.file(i).name().c_str());
        }
    }

    return true;
}

bool LoadPbFromFile(const std::string& path) {
    std::ifstream fs(path, std::ios::binary);
    if (!fs.is_open()) {
        DebugPrintA("unable to open file: %s\n", path.c_str());
        return false;
    }

    fs.seekg(0, std::ios::end);
    std::streamsize size = fs.tellg();
    fs.seekg(0, std::ios::beg);

    std::string buffer(size, '\0');
    if (!fs.read(buffer.data(), size)) {
        DebugPrintA("unable to read file: %s\n", path.c_str());
        return false;
    }

    return LoadPbFromMemory(buffer.data(), static_cast<int>(size));
}

bool DeserializeToJson(int32_t cmdId, const void* binaryData, int dataLen, std::string& outJson) {
    if (!binaryData || dataLen <= 0) {
        return false;
    }

    auto cmdName = GetMsgName(cmdId);
    auto it = kNameToProto.find(cmdName);
    if (it == kNameToProto.end()) {
        DebugPrintA("Unknown packet ID: %d\n", cmdId);
        return false;
    }

    const std::string& protoName = it->second;

    const google::protobuf::Descriptor* desc = g_descriptorPool.FindMessageTypeByName(protoName);
    if (!desc) {
        DebugPrintA("Descriptor not found: %s\n", protoName.c_str());
        return false;
    }

    const google::protobuf::Message* prototype = g_factory.GetPrototype(desc);

    if (!prototype) {
        DebugPrintA("Unable to obtain proto: %s\n", protoName.c_str());
        return false;
    }

    std::unique_ptr<google::protobuf::Message> msg(prototype->New());

    if (!msg->ParseFromArray(binaryData, dataLen)) {
        DebugPrintA("Unable to parse data: %s, data: %s\n", protoName.c_str(), ByteArrayToHex((uint8_t*)binaryData, (size_t)dataLen).c_str());
        return false;
    }

    google::protobuf::util::JsonPrintOptions options;
    options.add_whitespace = true;
    if (!google::protobuf::util::MessageToJsonString(*msg, &outJson, options).ok()) {
        DebugPrintA("Failed to serialize cmdId: %d to JSON\n", cmdId);
        return false;
    }

    return true;
}

const std::string& GetMsgName(int32_t cmdId)
{
    static const std::string unknown = "UnknownPacket";
    auto it = kIdToName.find(cmdId);
    return it != kIdToName.end() ? it->second : unknown;
}

bool DeserializeToPacketList(int32_t cmdId, const void* binaryData, int dataLen, bool fromServer, std::vector<Packet>& outPackets)
{
    std::string jsonStr;
    auto succ = DeserializeToJson(cmdId, binaryData, dataLen, jsonStr);
    if (!succ) {
        return false;
    }

    Json::Value obj;

    Json::CharReaderBuilder builder;
    std::string errs;
    std::istringstream ss(jsonStr);
    if (!Json::parseFromStream(builder, ss, &obj, &errs)) {
        obj = Json::nullValue;
    }

    Packet packet{};
    packet.time = std::chrono::duration_cast<std::chrono::milliseconds>(std::chrono::system_clock::now().time_since_epoch()).count();
    packet.fromServer = fromServer;
    packet.packetId = cmdId;
    packet.packetName = GetMsgName(packet.packetId);
    packet.object = obj;
    packet.raw.assign((char*)binaryData, dataLen);

    outPackets.push_back(std::move(packet));

    if (!obj.isObject() || !obj.isMember("NextPackage") || !obj["NextPackage"].isString()) {
        return true;
    }

    std::string nextPackage = obj["NextPackage"].asString();
    std::string nextData = Base64Decode(nextPackage);
    if (nextData.size() < 3) {
        DebugPrintA("NextPackage too small\n");
        return false;
    }

    uint16_t nextMsgId;
    memcpy(&nextMsgId, nextData.data(), 2);
    nextMsgId = ntohs(nextMsgId);

    const void* nextPayload = nextData.data() + 2;
    int nextLen = static_cast<int>(nextData.size() - 2);

    return DeserializeToPacketList(nextMsgId, nextPayload, nextLen, fromServer, outPackets);
}
