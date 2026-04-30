#pragma once

#include <string>
#include <mutex>

extern std::string g_activeClientID;
extern std::mutex g_activeMutex;

bool StartServer(unsigned short port);

bool StopServer();

void PushEvent(const std::string& json, const std::string& clientID);
