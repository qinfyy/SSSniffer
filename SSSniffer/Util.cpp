#include "pch.h"
#include "Util.h"
#include <string>
#include <iomanip>
#include "PrintHelper.h"

static const std::string BASE64_CHARS = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

std::string Base64Encode(const std::string& input) {
    if (input.empty())
        return "";

    std::string output;
    int val = 0, valb = -6;

    for (unsigned char c : input) {
        val = (val << 8) + c;
        valb += 8;
        while (valb >= 0) {
            output.push_back(BASE64_CHARS[(val >> valb) & 0x3F]);
            valb -= 6;
        }
    }

    if (valb > -6) {
        output.push_back(BASE64_CHARS[((val << 8) >> (valb + 8)) & 0x3F]);
    }

    while (output.size() % 4) {
        output.push_back('=');
    }

    return output;
}

std::string Base64Decode(const std::string& input) {
    if (input.empty())
        return "";

    std::vector<int> T(256, -1);
    for (int i = 0; i < 64; i++) {
        T[static_cast<unsigned char>(BASE64_CHARS[i])] = i;
    }

    std::string output;
    int val = 0, valb = -8;

    for (unsigned char c : input) {
        if (c == '=')
            break;

        val = (val << 6) + T[c];
        valb += 6;

        if (valb >= 0) {
            output.push_back(static_cast<char>((val >> valb) & 0xFF));
            valb -= 8;
        }
    }

    return output;
}

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
    if (!data || len == 0)
        return "";

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
    if (!arr)
        return "";

    return ByteArrayToHex(arr->vector, arr->max_length);
}