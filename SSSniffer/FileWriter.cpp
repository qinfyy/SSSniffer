#include "pch.h"
#include "FileWriter.h"
#include <unordered_map>
#include <mutex>
#include <string>
#include <cstdio>
#include <cstdarg>
#include <cstdlib>
#include <ctime>

struct LogFileEntry {
    FILE* file = nullptr;
    std::mutex mtx;
};

static std::string g_PacketCaptureFile;
static std::once_flag g_InitFlag;

void InitPacketCaptureFile() {
    time_t t = time(NULL);
    struct tm tm;
    localtime_s(&tm, &t);

    char buf[128];
    snprintf(buf, sizeof(buf), "PacketCapture_%02d-%02d-%02d_%02d-%02d-%02d.json", tm.tm_year % 100, tm.tm_mon + 1, tm.tm_mday, tm.tm_hour, tm.tm_min, tm.tm_sec);

    g_PacketCaptureFile = buf;
}

static std::unordered_map<std::string, LogFileEntry> g_LogTable;
static std::mutex g_TableMutex;

bool WriteToFile(const char* fmt, ...)
{
    std::call_once(g_InitFlag, InitPacketCaptureFile);

    LogFileEntry* entry = nullptr;

    {
        std::lock_guard lock(g_TableMutex);
        auto& ref = g_LogTable[g_PacketCaptureFile];
        entry = &ref;

        if (!ref.file) {
            fopen_s(&ref.file, g_PacketCaptureFile.c_str(), "w");
            if (!ref.file)
                return false;
        }
    }

    std::lock_guard lock(entry->mtx);

    char stackBuf[512];
    va_list args;
    va_start(args, fmt);
    int needed = vsnprintf(stackBuf, sizeof(stackBuf), fmt, args);
    va_end(args);

    if (needed < 0)
        return false;

    const char* writeBuf = nullptr;
    bool useHeap = false;

    if ((size_t)needed < sizeof(stackBuf)) {
        writeBuf = stackBuf;
    }
    else {
        char* heapBuf = (char*)malloc(needed + 1);
        if (!heapBuf)
            return false;

        va_start(args, fmt);
        vsnprintf(heapBuf, needed + 1, fmt, args);
        va_end(args);

        writeBuf = heapBuf;
        useHeap = true;
    }

    if (fputs(writeBuf, entry->file) < 0) {
        if (useHeap) free((void*)writeBuf);
        return false;
    }
    fflush(entry->file);

    if (useHeap)
        free((void*)writeBuf);

    return true;
}
