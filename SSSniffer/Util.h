#pragma once
#include <string>
#include <vector>
#include <sstream>
#include "Types.h"

std::wstring Il2cppToWstring(Il2CppString* str);

bool ReplaceIl2CppStringChars(Il2CppString* target, const std::wstring& ws);

Il2CppString* CreateIl2CppString(const std::wstring& ws, Il2CppString* original);

std::string ByteArrayToHex(const uint8_t* data, size_t len);

std::string ByteArrayToHex(Byte__Array* arr);
