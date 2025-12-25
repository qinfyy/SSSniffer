#include "pch.h"
#include "HookManager.h"
#include "Hooks.h"
#include "PrintHelper.h"
#include <detours.h>
#include "Memory.h"

void InstallHooks() {
    auto base = GetModuleHandleA("GameAssembly.dll");

    uintptr_t bmAddr = Scan(base, "4C 89 44 24 ?? 48 89 4C 24 ?? 53 56 57 41 54 41 55 41 56 41 57 48 81 EC ?? ?? ?? ?? 4D 8B F8 4C 8B E2");
    uintptr_t rmAddr = Scan(base, "48 89 5C 24 ?? 48 89 74 24 ?? 57 41 56 41 57 48 83 EC ?? 45 8B F1 41 8B F8 48 8B F2 80 3D ?? ?? ?? ?? ??");
    
    if (!bmAddr) {
        DebugPrintA("[ERROR] Failed to find bm.\n");
        return;
    }

    if (!rmAddr) {
        DebugPrintA("[ERROR] Failed to find rm.\n");
        return;
    }

    DebugPrintA("[INFO] bm: %lx\n", bmAddr);
    DebugPrintA("[INFO] rm: %lx\n", rmAddr);

    o_readMessage = (void*)(rmAddr);
    o_BuildMessage = (void*)(bmAddr);

    DetourTransactionBegin();
    DetourUpdateThread(GetCurrentThread());

    DetourAttach(&(PVOID&)o_readMessage, readMessage_Hook);
    DetourAttach(&(PVOID&)o_BuildMessage, BuildMessage_Hook);

    DetourTransactionCommit();

    DebugPrintLockA("[INFO] Hooks installed\n");
}

void UninstallHooks() {
    DetourTransactionBegin();
    DetourUpdateThread(GetCurrentThread());

	// here need uninstall hooks

    DetourTransactionCommit();
}

DWORD WINAPI WaitForGAModule(LPVOID) {
    DebugPrintLockA("[INFO] Waiting for GameAssembly.dll...\n");
    while (!GetModuleHandleA("GameAssembly.dll")) Sleep(200);

    DebugPrintLockA("[INFO] GameAssembly.dll loaded, installing hooks...\n");
    InstallHooks();
    return 0;
}
