#![allow(non_snake_case, unsafe_op_in_unsafe_fn, dead_code)]
#[cfg(not(all(windows, target_arch = "x86_64")))]
compile_error!("SSSniffer requires x86_64 Windows");

mod HookManager;
mod PrintHelper;
mod Util;
mod Version;
mod capture;
mod hooks;
mod protocol;
mod server;

use std::ffi::c_void;
use windows::{
    Win32::{
        Foundation::{CloseHandle, HINSTANCE, HMODULE, HWND, TRUE},
        System::{
            LibraryLoader::DisableThreadLibraryCalls,
            SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH},
            Threading::{CreateThread, ExitProcess, Sleep, THREAD_CREATION_FLAGS},
        },
    },
    core::BOOL,
};

fn IsUnityCrashHandler() -> bool {
    let Ok(path) = Util::module_path(None) else {
        return false;
    };
    path.file_name().is_some_and(|name| {
        let name = name.to_string_lossy();
        name.eq_ignore_ascii_case("UnityCrashHandler64.exe")
            || name.eq_ignore_ascii_case("UnityCrashHandler32.exe")
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllMain(module: HMODULE, reason: u32, reserved: *mut c_void) -> BOOL {
    if reason == DLL_PROCESS_ATTACH {
        if IsUnityCrashHandler() {
            ExitProcess(0);
        }
        Version::LoadVersionDll();
        let _ = DisableThreadLibraryCalls(module);
        if let Ok(thread) = CreateThread(
            None,
            0,
            Some(HookManager::WaitForGAModule),
            None,
            THREAD_CREATION_FLAGS(0),
            None,
        ) {
            let _ = CloseHandle(thread);
        }
    } else if reason == DLL_PROCESS_DETACH && reserved.is_null() {
        let _ = hooks::uninstall();
    }
    TRUE
}

#[unsafe(no_mangle)]
pub unsafe extern "system" fn EntryPoint(_: HWND, _: HINSTANCE, _: *mut u8, _: i32) {
    Sleep(u32::MAX);
}
