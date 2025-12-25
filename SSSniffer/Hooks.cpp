#include "pch.h"
#include "Hooks.h"
#include "PrintHelper.h"
#include "FileWriter.h"
#include <chrono>

HttpNetMsg* readMessage_Hook(
    void* thisPtr,
    Byte__Array* messageBuffer,
    int32_t offset,
    int32_t length,
    void* method) {

    HttpNetMsg* ret = ((HttpNetMsg* (*)(void*, void*, int32_t, int32_t, void*))o_readMessage)(
        thisPtr, messageBuffer, offset, length, method
        );

    auto now_ms = std::chrono::duration_cast<std::chrono::milliseconds>(
        std::chrono::system_clock::now().time_since_epoch()
    ).count();

    if (ret) {
        DebugPrintLockA("RECV Time: %lld, msgId: %d\n", now_ms, ret->fields.msgId);
        WriteFile(".\\RECV.txt", "RECV Time: %lld, msgId: %d\n", now_ms, ret->fields.msgId);

        if (ret->fields.msgBody) {
            DebugPrintLockA("      msgBody len: %d\n", ret->fields.msgBody->max_length);
            DebugPrintLockA("      msgBody(hex): %s\n", ByteArrayToHex(ret->fields.msgBody->vector, ret->fields.msgBody->max_length).c_str());
            DebugPrintLockA("      receiveMsgId: %d\n", ret->fields.receiveMsgId);

            WriteFile(".\\RECV.txt", "      msgBody len: %d\n", ret->fields.msgBody->max_length);
            WriteFile(".\\RECV.txt", "      msgBody(hex): %s\n", ByteArrayToHex(ret->fields.msgBody->vector, ret->fields.msgBody->max_length).c_str());
            WriteFile(".\\RECV.txt", "      receiveMsgId: %d\n", ret->fields.receiveMsgId);
        }
    }
    else {
        DebugPrintLockA("RECV msg is NULL\n");
        WriteFile(".\\RECV.txt", "RECV msg is NULL\n");
    }

    WriteFile(".\\RECV.txt", "\n");

    return ret;
}

bool BuildMessage_Hook(
    void* thisPtr,
    HttpNetMsg* msg,
    void* data,
    void* method) {

    auto now_ms = std::chrono::duration_cast<std::chrono::milliseconds>(
        std::chrono::system_clock::now().time_since_epoch()
    ).count();

    if (msg) {
        DebugPrintLockA("SEND Time: %lld, msgId: %d\n", now_ms, msg->fields.msgId);
        WriteFile(".\\SEND.txt", "SEND Time: %lld, msgId: %d\n", now_ms, msg->fields.msgId);

        if (msg->fields.msgBody)
        {
            DebugPrintLockA("      msgBody len: %d\n", msg->fields.msgBody->max_length);
            DebugPrintLockA("      msgBody(hex): %s\n", ByteArrayToHex(msg->fields.msgBody->vector, msg->fields.msgBody->max_length).c_str());

            WriteFile(".\\SEND.txt", "      msgBody len: %d\n", msg->fields.msgBody->max_length);
            WriteFile(".\\SEND.txt", "      msgBody(hex): %s\n", ByteArrayToHex(msg->fields.msgBody->vector, msg->fields.msgBody->max_length).c_str());
        }

        DebugPrintLockA("      receiveMsgId: %d\n", msg->fields.receiveMsgId);
        WriteFile(".\\SEND.txt", "      receiveMsgId: %d\n", msg->fields.receiveMsgId);
    }
    else
    {
        DebugPrintLockA("SEND msg is NULL\n");
        WriteFile(".\\SEND.txt", "SEND msg is NULL\n");
    }

    WriteFile(".\\SEND.txt", "\n");

    bool ret = ((bool (*)(void*, HttpNetMsg*, void*, void*))o_BuildMessage)(thisPtr, msg, data, method);

    return ret;
}
