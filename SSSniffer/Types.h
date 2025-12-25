#pragma once

struct Il2CppString // sizeof=0x58
{
    void* m_pClass;
    void* monitor;
    int32_t length;
    wchar_t chars[32];
    // padding byte
    // padding byte
    // padding byte
    // padding byte
};

typedef uintptr_t il2cpp_array_size_t; // XREF: Il2CppArrayBounds/r
struct Byte__Array // sizeof=0x40
{
    struct Byte__Array__Class* klass;
    struct MonitorData* monitor;
    struct Il2CppArrayBounds* bounds;
    il2cpp_array_size_t max_length;
    uint8_t vector[32];
};




//Game 
struct __declspec(align(8)) HttpNetMsg__Fields // sizeof=0x20
{                                       // XREF: HttpNetMsg/r
    int16_t msgId;
    // padding byte
    // padding byte
    // padding byte
    // padding byte
    // padding byte
    // padding byte
    struct Byte__Array* msgBody;
    void* callback;
    int16_t receiveMsgId;
    // padding byte
    // padding byte
    // padding byte
    // padding byte
    // padding byte
    // padding byte
};

struct HttpNetMsg // sizeof=0x30
{
    void* klass; //HttpNetMsg__Class*
    void* monitor; //struct MonitorData*
    struct HttpNetMsg__Fields fields;
};
