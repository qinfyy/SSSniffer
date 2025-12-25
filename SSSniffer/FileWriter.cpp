#include "pch.h"
#include "FileWriter.h"
#include <unordered_map>
#include <mutex>
#include <string>
#include <cstdio>
#include <cstdarg>
#include <cstdlib>

struct LogFileEntry {
    FILE* file = nullptr;
    std::mutex mtx;
};

static std::unordered_map<std::string, LogFileEntry> g_LogTable;
static std::mutex g_TableMutex;

bool WriteFile(const char* path, const char* fmt, ...)
{
    if (!path || !fmt)
        return false;

    LogFileEntry* entry = nullptr;

    {
        std::lock_guard lock(g_TableMutex);
        auto& ref = g_LogTable[path];
        entry = &ref;

        if (!ref.file) {
            fopen_s(&ref.file, path, "a+");
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
