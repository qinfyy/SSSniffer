#pragma once

#include <string>

bool StartServer(unsigned short port);

bool StopServer();

void PushEvent(const std::string& msg);
