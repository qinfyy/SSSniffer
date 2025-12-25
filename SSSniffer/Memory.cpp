#include "pch.h"
#include "Memory.h"
#include <string>

uintptr_t Scan(HMODULE moduleBase, LPCSTR pattern)
{
    if (!moduleBase)
        return 0;

    PIMAGE_DOS_HEADER dosHeader = (PIMAGE_DOS_HEADER)moduleBase;
    PIMAGE_NT_HEADERS ntHeaders = (PIMAGE_NT_HEADERS)((BYTE*)moduleBase + dosHeader->e_lfanew);
    DWORD sizeOfImage = ntHeaders->OptionalHeader.SizeOfImage;

    int* patternBytes = (int*)HeapAlloc(GetProcessHeap(), HEAP_ZERO_MEMORY, (lstrlenA(pattern) / 3 + 1) * sizeof(int));
    if (!patternBytes) {
        return 0;
    }

    int patternIndex = 0;
    const char* start = pattern;
    const char* end = start + lstrlenA(pattern);

    for (const char* current = start; current < end; ++current) {
        if (*current == '?') {
            ++current;
            if (*current == '?')
                ++current;
            patternBytes[patternIndex++] = -1;
        }
        else {
            patternBytes[patternIndex++] = strtoul(current, (char**)&current, 16);
        }
    }

    BYTE* scanBytes = (BYTE*)moduleBase;
    DWORD patternSize = patternIndex;

    for (DWORD i = 0; i < sizeOfImage - patternSize; ++i) {
        bool found = true;
        for (DWORD j = 0; j < patternSize; ++j) {
            if (scanBytes[i + j] != patternBytes[j] && patternBytes[j] != -1) {
                found = false;
                break;
            }
        }

        if (found) {
            HeapFree(GetProcessHeap(), 0, patternBytes);
            return (uintptr_t)&scanBytes[i];
        }
    }

    HeapFree(GetProcessHeap(), 0, patternBytes);
    return 0;
}
