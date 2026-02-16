#include "pch.h"
#include "Hooks.h"
#include "PrintHelper.h"
#include "FileWriter.h"
#include <chrono>
#include "BinaryDeserializer.h"
#include "FrontendServer.h"
#include "Util.h"

HttpNetMsg* readMessage_Hook(void* thisPtr, Byte__Array* messageBuffer, int32_t offset, int32_t length, void* method) {
    HttpNetMsg* ret = ((HttpNetMsg * (*)(void*, void*, int32_t, int32_t, void*))o_readMessage)(thisPtr, messageBuffer, offset, length, method);

    if (!ret)
        return ret;

    Packet pkt{};
    pkt.time = std::chrono::duration_cast<std::chrono::milliseconds>(std::chrono::system_clock::now().time_since_epoch()).count();
    pkt.fromServer = true;
    pkt.packetId = ret->fields.msgId;
    pkt.packetName = GetMsgName(ret->fields.msgId);

    if (ret->fields.msgBody && ret->fields.msgBody->vector) {
        auto len = ret->fields.msgBody->max_length;
        const uint8_t* data = ret->fields.msgBody->vector;

        if (len > 15) {
            const uint8_t* p = data + len;
            uint8_t acc = 0;
            for (int i = 0; i < 16; ++i) {
                acc |= *(--p);
            }
            len -= (acc == 0) * 16;
        }

        pkt.raw.assign(data, data + len);

        std::string jsonStr;
        try {
            if (DeserializeToJson(pkt.packetId, pkt.raw.data(), pkt.raw.size(), jsonStr)) {
                pkt.object = nlohmann::json::parse(jsonStr);
            }
            else {
                pkt.object = nullptr;
            }
        }
        catch (const std::exception& e) {
            DebugPrintA("[rm] Exception: %s\n", e.what());
            pkt.object = nullptr;
        }
    }

    nlohmann::json j = {
        {"time", pkt.time},
        {"fromServer", pkt.fromServer},
        {"packetId", pkt.packetId},
        {"packetName", pkt.packetName},
        {"object", pkt.object},
        {"raw", Base64Encode(pkt.raw)}
    };

    std::string output = j.dump(4);

    WriteToFile("%s\n", output.c_str());
    PushEvent(output);

    return ret;
}

bool BuildMessage_Hook(void* thisPtr, HttpNetMsg* msg, void* data, void* method){
    if (!msg)
        return ((bool (*)(void*, HttpNetMsg*, void*, void*))o_BuildMessage)(thisPtr, msg, data, method);

    Packet pkt{};
    pkt.time = std::chrono::duration_cast<std::chrono::milliseconds>(std::chrono::system_clock::now().time_since_epoch()).count();
    pkt.fromServer = false;
    pkt.packetId = msg->fields.msgId;
    pkt.packetName = GetMsgName(msg->fields.msgId);

    if (msg->fields.msgBody && msg->fields.msgBody->vector) {
        auto len = msg->fields.msgBody->max_length;

        pkt.raw.assign(msg->fields.msgBody->vector, msg->fields.msgBody->vector + len);

        std::string jsonStr;
        try {
            if (DeserializeToJson(pkt.packetId, pkt.raw.data(), pkt.raw.size(), jsonStr))
            {
                    pkt.object = nlohmann::json::parse(jsonStr);
            }
            else {
                pkt.object = nullptr;
            }
        } catch (const std::exception& e) {
            DebugPrintA("[bm] Exception: %s\n", e.what());
            pkt.object = nullptr;
        }
    }

    nlohmann::json j = {
        {"time", pkt.time},
        {"fromServer", pkt.fromServer},
        {"packetId", pkt.packetId},
        {"packetName", pkt.packetName},
        {"object", pkt.object},
        {"raw", Base64Encode(pkt.raw)}
    };

    std::string output = j.dump(4);

    WriteToFile("%s\n", output.c_str());
    PushEvent(output);

    return ((bool (*)(void*, HttpNetMsg*, void*, void*))o_BuildMessage)(thisPtr, msg, data, method);
}
