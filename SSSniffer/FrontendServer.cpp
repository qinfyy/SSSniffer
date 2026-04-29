#include "pch.h"
#include "FrontendServer.h"
#include <winsock2.h>
#include <iostream>
#include <sstream>
#include <fstream>
#include <unordered_set>
#include <algorithm>
#include <Urlmon.h>
#include <filesystem>
#include <thread>
#include <mutex>
#include <vector>
#include <condition_variable>
#include <queue>
#include <chrono>
#include "PrintHelper.h"
//#include "../frontend/CppOutput/EmbedFiles.h"

#pragma comment(lib, "ws2_32.lib")
#pragma comment(lib, "urlmon.lib")

static SOCKET g_listenSocket = INVALID_SOCKET;
static bool g_running = false;

static std::unordered_set<SOCKET> g_sseClients;
static std::mutex g_sseMutex;
static std::condition_variable g_sseCv;
static std::queue<std::string> g_eventQueue;

static void SendResponse(SOCKET clientSocket, const std::string& body, const std::string& contentType = "text/plain") {
    std::ostringstream response;
    response << "HTTP/1.1 200 OK\r\n"
        << "Content-Type: " << contentType << "\r\n"
        << "Content-Length: " << body.size() << "\r\n"
        << "Connection: close\r\n\r\n"
        << body;

    std::string respStr = response.str();
    send(clientSocket, respStr.c_str(), static_cast<int>(respStr.size()), 0);
    closesocket(clientSocket);
}

static std::string GetMimeType(const std::string& path) {
    std::string mime = "application/octet-stream";
    if (!path.empty()) {
        wchar_t wPath[MAX_PATH];
        MultiByteToWideChar(CP_UTF8, 0, path.c_str(), -1, wPath, MAX_PATH);

        LPWSTR mimeType = NULL;
        HRESULT hr = FindMimeFromData(NULL, wPath, NULL, 0, NULL, 0, &mimeType, 0);
        if (SUCCEEDED(hr) && mimeType) {
            char buffer[256] = { 0 };
            WideCharToMultiByte(CP_UTF8, 0, mimeType, -1, buffer, 256, NULL, NULL);
            mime = buffer;
            CoTaskMemFree(mimeType);
        }
    }
    return mime;
}

static void HandleStaticFile(SOCKET clientSocket, const std::string& urlPath) {
    std::string path = "./frontend/public" + urlPath;

    if (path.back() == '/')
        path += "index.html";

    if (std::filesystem::exists(path)) {
        std::ifstream file(path, std::ios::binary);
        if (file) {
            std::string content((std::istreambuf_iterator<char>(file)), std::istreambuf_iterator<char>());
            std::string mime = GetMimeType(path);
            SendResponse(clientSocket, content, mime);
            return;
        }
    }

#ifdef USE_EMBEDDED_FILES
    int embedLen = 0;
    const char* embedData = nullptr;
    bool embedOk = false;

    GetEmbedFileData(path.c_str(), embedLen, &embedData, embedOk);

    if (embedOk && embedData && embedLen > 0) {
        std::string mime = GetMimeType(path);
        std::string content(embedData, embedLen);
        SendResponse(clientSocket, content, mime);
        return;
    }
#endif

    SendResponse(clientSocket, "File Not Found", "text/plain");
}

static bool RecvRequest(SOCKET clientSocket, std::string& outHeader, std::string& outBody) {
    outHeader.clear();
    outBody.clear();

    const int BUF_SIZE = 4096;
    std::string requestData;
    requestData.reserve(BUF_SIZE);

    auto headerEnd = std::string::npos;

    while ((headerEnd = requestData.find("\r\n\r\n")) == std::string::npos) {
        char buf[BUF_SIZE];
        int r = recv(clientSocket, buf, BUF_SIZE, 0);
        if (r <= 0) return false;
        requestData.append(buf, r);
    }

    outHeader = requestData.substr(0, headerEnd + 4);

    int contentLength = 0;
    auto clPos = outHeader.find("Content-Length:");
    if (clPos != std::string::npos) {
        auto lineEnd = outHeader.find("\r\n", clPos);
        std::string clLine = outHeader.substr(clPos, lineEnd - clPos);
        auto colon = clLine.find(":");
        if (colon != std::string::npos) {
            contentLength = std::stoi(clLine.substr(colon + 1));
        }
    }

    if (contentLength > 0) {
        outBody.reserve(contentLength);

        auto bodyStart = headerEnd + 4;
        if (requestData.size() > bodyStart) {
            outBody.append(requestData.data() + bodyStart, requestData.size() - bodyStart);
        }

        while (outBody.size() < (size_t)contentLength) {
            char buf[BUF_SIZE];
            int r = recv(clientSocket, buf, BUF_SIZE, 0);
            if (r <= 0) return false;

            size_t toCopy = min((size_t)r, (size_t)(contentLength - outBody.size()));
            outBody.append(buf, toCopy);
        }
    }

    return true;
}

static void HandleApiStream(SOCKET clientSocket) {
    {
        std::lock_guard<std::mutex> lock(g_sseMutex);
        g_sseClients.insert(clientSocket);
    }

    std::string headers =
        "HTTP/1.1 200 OK\r\n"
        "Content-Type: text/event-stream\r\n"
        "Cache-Control: no-cache\r\n"
        "Connection: keep-alive\r\n"
        "Access-Control-Allow-Origin: *\r\n\r\n";

    send(clientSocket, headers.c_str(), (int)headers.size(), 0);

    std::thread([clientSocket]() {
        while (g_running) {
            std::unique_lock<std::mutex> lock(g_sseMutex);
            g_sseCv.wait(lock, [] { return !g_eventQueue.empty() || !g_running; });

            if (!g_running) break;

            std::queue<std::string> snapshot = g_eventQueue;
            lock.unlock();

            while (!snapshot.empty()) {
                std::string msg = snapshot.front();
                snapshot.pop();

                std::ostringstream ss;
                ss << "event: packetNotify\n";
                ss << "data: " << msg << "\n\n";

                if (send(clientSocket, ss.str().c_str(), (int)ss.str().size(), 0) <= 0) {
                    closesocket(clientSocket);
                    std::lock_guard<std::mutex> lock2(g_sseMutex);
                    g_sseClients.erase(clientSocket);
                    return;
                }
            }
        }

        closesocket(clientSocket);
        std::lock_guard<std::mutex> lock2(g_sseMutex);
        g_sseClients.erase(clientSocket);
        }).detach();
}

static void HandleApiStart(SOCKET clientSocket) {
    // TODO
    SendResponse(clientSocket, "OK");
}

static void HandleApiStop(SOCKET clientSocket) {
    // TODO
    SendResponse(clientSocket, "OK");
}

static std::vector<std::string> ReadJsonBlocks(const std::string& content) {
    std::vector<std::string> result;
    std::istringstream iss(content);
    std::string line;
    std::string currentBlock;
    bool inBlock = false;

    while (std::getline(iss, line)) {
        if (line.empty()) {
            continue;
        }

        if (line[0] == '{') {
            inBlock = true;
            currentBlock.clear();
        }

        if (inBlock) {
            currentBlock += line + "\n";
        }

        if (line[0] == '}' && inBlock) {
            inBlock = false;
            result.push_back(currentBlock);
        }
    }

    return result;
}

static void HandleApiUpload(SOCKET clientSocket, const std::string& body) {
    std::istringstream iss(body);
    std::string line;
    if (!std::getline(iss, line)) {
        DebugPrintLockA("Bad Request");
        SendResponse(clientSocket, "Bad Request");
        return;
    }
    std::string boundary = line;
    boundary.erase(std::remove(boundary.begin(), boundary.end(), '\r'), boundary.end());

    std::ostringstream fileContent;
    bool inFile = false;

    while (std::getline(iss, line)) {

        if (!line.empty() && line.back() == '\r') {
            line.pop_back();
        }

        if (line == boundary + "--") {
            break;
        }

        if (inFile) {
            fileContent << line << "\n";
        }

        if (!inFile && line.empty()) {
            inFile = true;
        }
    }

    std::string content = fileContent.str();
    if (!content.empty() && content.back() == '\n') {
        content.pop_back();
    }

    auto packets = ReadJsonBlocks(content);

    for (const auto& packet : packets) {
        PushEvent(packet);
    }

    SendResponse(clientSocket, "OK");
}

static void HandleApiGetConfig(SOCKET clientSocket) {
    // TODO
    SendResponse(clientSocket, "OK");
}

static void HandleApiSetConfig(SOCKET clientSocket) {
    // TODO
    SendResponse(clientSocket, "OK");
}

static void HandleClient(SOCKET clientSocket) {
    try {
        std::string header, body;
        if (!RecvRequest(clientSocket, header, body)) {
            closesocket(clientSocket);
            return;
        }

        if (header.find("GET /api/start") == 0) {
            HandleApiStart(clientSocket);
        }
        else if (header.find("GET /api/stop") == 0) {
            HandleApiStop(clientSocket);
        }
        else if (header.find("POST /api/upload") == 0) {
            HandleApiUpload(clientSocket, body);
        }
        else if (header.find("GET /api/stream") == 0) {
            HandleApiStream(clientSocket);
        }
        else if (header.find("GET /api/GetConfig") == 0) {
            HandleApiGetConfig(clientSocket);
        }
        else if (header.find("GET /api/SetConfig") == 0) {
            HandleApiSetConfig(clientSocket);
        }
        else {
            auto qpos = header.find(" ");
            auto qpos2 = header.find(" ", qpos + 1);
            std::string urlPath = "/";
            if (qpos != std::string::npos && qpos2 != std::string::npos) {
                urlPath = header.substr(qpos + 1, qpos2 - qpos - 1);
                auto qm = urlPath.find('?');
                if (qm != std::string::npos) urlPath = urlPath.substr(0, qm);
            }
            HandleStaticFile(clientSocket, urlPath);
        }
    }
    catch (...) {
        closesocket(clientSocket);
    }
}

static void AcceptLoop() {
    while (g_running) {
        SOCKET clientSocket = accept(g_listenSocket, nullptr, nullptr);
        if (clientSocket != INVALID_SOCKET) {
            std::thread(HandleClient, clientSocket).detach();
        }
    }
}

void PushEvent(const std::string& json) {
    std::lock_guard<std::mutex> lock(g_sseMutex);

    std::ostringstream ss;
    ss << "event: packetNotify\n";

    std::istringstream iss(json);
    std::string line;
    while (std::getline(iss, line)) {
        if (!line.empty() && line.back() == '\r') {
            line.pop_back();
        }
        ss << "data: " << line << "\n";
    }
    ss << "\n";

    std::string data = ss.str();

    for (auto it = g_sseClients.begin(); it != g_sseClients.end();) {
        SOCKET s = *it;
        if (send(s, data.c_str(), (int)data.size(), 0) <= 0) {
            closesocket(s);
            it = g_sseClients.erase(it);
        }
        else {
            ++it;
        }
    }
}

bool StartServer(unsigned short port) {
    WSADATA wsaData;
    if (WSAStartup(MAKEWORD(2, 2), &wsaData) != 0) {
        int errorCode = WSAGetLastError();
        DebugPrintA("[FrontendServer] WSAStartup failed: %d\n", errorCode);
        return false;
    }

    g_listenSocket = socket(AF_INET, SOCK_STREAM, IPPROTO_TCP);
    if (g_listenSocket == INVALID_SOCKET) {
        int errorCode = WSAGetLastError();
        DebugPrintA("[FrontendServer] Create socket failed: %d\n", errorCode);
        return false;
    }

    // 设置 SO_REUSEADDR 支持快速重启
    BOOL opt = TRUE;
    if (setsockopt(g_listenSocket, SOL_SOCKET, SO_REUSEADDR, (char*)&opt, sizeof(opt)) == SOCKET_ERROR) {
        int errorCode = WSAGetLastError();
        DebugPrintA("[FrontendServer] setsockopt SO_REUSEADDR failed: %d\n", errorCode);
    }

    sockaddr_in serverAddr{};
    serverAddr.sin_family = AF_INET;
    serverAddr.sin_addr.s_addr = INADDR_ANY;
    serverAddr.sin_port = htons(port);

    if (bind(g_listenSocket, (sockaddr*)&serverAddr, sizeof(serverAddr)) == SOCKET_ERROR) {
        int errorCode = WSAGetLastError();
        DebugPrintA("[FrontendServer] Bind failed: %d\n", errorCode);
        closesocket(g_listenSocket);
        return false;
    }

    if (listen(g_listenSocket, SOMAXCONN) == SOCKET_ERROR) {
        int errorCode = WSAGetLastError();
        DebugPrintA("[FrontendServer] Listen failed: %d\n", errorCode);
        closesocket(g_listenSocket);
        return false;
    }

    g_running = true;
    std::thread(AcceptLoop).detach();

    DebugPrintA("[FrontendServer] Listening on port %d\n", port);
    return true;
}

bool StopServer() {
    if (!g_running && g_listenSocket == INVALID_SOCKET) {
        return true;
    }

    g_running = false;
    g_sseCv.notify_all();
    bool succ = true;

    if (g_listenSocket != INVALID_SOCKET) {
        if (closesocket(g_listenSocket) == SOCKET_ERROR) {
            int err = WSAGetLastError();
            DebugPrintA("[FrontendServer] closesocket failed: %d\n", err);
            succ = false;
        }
        g_listenSocket = INVALID_SOCKET;
    }

    {
        std::lock_guard<std::mutex> lock(g_sseMutex);
        for (SOCKET s : g_sseClients) {
            if (closesocket(s) == SOCKET_ERROR) {
                int err = WSAGetLastError();
                DebugPrintA("[FrontendServer] closesocket client failed: %d\n", err);
                succ = false;
            }
        }
        g_sseClients.clear();
    }

    {
        std::lock_guard<std::mutex> lock(g_sseMutex);
        std::queue<std::string> empty;
        std::swap(g_eventQueue, empty);
    }

    WSACleanup();
    DebugPrintA("[FrontendServer] Server stopped\n");
    return succ;
}
