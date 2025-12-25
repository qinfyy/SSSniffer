#include "pch.h"
#include "Util.h"
#include <string>
#include <codecvt>
#include <iomanip>

std::wstring Il2cppToWstring(Il2CppString* str) {
    if (!str || str->length <= 0)
        return {};

    return std::wstring(str->chars, str->length);
}

Il2CppString* CreateIl2CppString(const std::wstring& ws, Il2CppString* original)
{
    if (!original) return nullptr;

    int32_t len = static_cast<int32_t>(ws.size());
    size_t size = sizeof(Il2CppString) + (len - 1) * sizeof(wchar_t);

    Il2CppString* newStr = (Il2CppString*)malloc(size);
    if (!newStr) return nullptr;

    newStr->m_pClass = original->m_pClass;
    newStr->monitor = nullptr;
    newStr->length = len;

    memcpy(newStr->chars, ws.c_str(), len * sizeof(wchar_t));

    return newStr;
}

bool ReplaceIl2CppStringChars(Il2CppString* target, const std::wstring& ws)
{
    if (!target)
        return false;

    const size_t capacity = static_cast<size_t>(target->length);

    if (ws.size() > capacity)
        return false;

    std::memcpy(target->chars, ws.c_str(), ws.size() * sizeof(wchar_t));

    if (ws.size() < capacity) {
        std::memset(target->chars + ws.size(), 0, (capacity - ws.size()) * sizeof(wchar_t));
    }

    target->length = static_cast<int32_t>(ws.size());
    return true;
}

std::string ByteArrayToHex(const uint8_t* data, size_t len)
{
    if (!data || len == 0) return "";

    std::ostringstream oss;
    oss << std::hex << std::setfill('0');
    for (size_t i = 0; i < len; ++i)
    {
        oss << std::setw(2) << static_cast<int>(data[i]);
    }
    return oss.str();
}

std::string ByteArrayToHex(Byte__Array* arr)
{
    if (!arr) return "";
    return ByteArrayToHex(arr->vector, arr->max_length);
}