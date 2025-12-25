#pragma once
#include "Util.h"

inline void* o_readMessage = nullptr;
inline void* o_BuildMessage = nullptr;

HttpNetMsg* readMessage_Hook(
    void* thisPtr,
    Byte__Array* messageBuffer,
    int32_t offset,
    int32_t length,
    void* method);

bool BuildMessage_Hook(
    void* thisPtr,
    HttpNetMsg* msg,
    void* data,
    void* method);
