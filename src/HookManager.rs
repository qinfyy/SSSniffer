use crate::{
    DebugPrintLockln,
    Util::module_path,
    capture, hooks,
    protocol::{Config, Protocol},
    server::{self, AppState},
};
use anyhow::{Context, Result};
use std::{ffi::c_void, panic::catch_unwind, sync::Arc, time::Duration};
use windows::{
    Win32::{
        Foundation::HMODULE,
        System::LibraryLoader::{
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_PIN,
            GetModuleHandleExW, GetModuleHandleW,
        },
    },
    core::{PCWSTR, w},
};

pub unsafe extern "system" fn WaitForGAModule(_: *mut c_void) -> u32 {
    match catch_unwind(start) {
        Ok(Ok(())) => 0,
        Ok(Err(error)) => {
            DebugPrintLockln!("[ERROR] {error:#}");
            1
        }
        Err(_) => {
            DebugPrintLockln!("[ERROR] Sniffer initialization panicked");
            1
        }
    }
}

fn start() -> Result<()> {
    let mut module = HMODULE::default();
    unsafe {
        // Background HTTP/capture threads require the DLL to remain loaded until process exit.
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_PIN,
            PCWSTR(WaitForGAModule as *const () as *const u16),
            &mut module,
        )?;
    }
    let directory = module_path(Some(module))?
        .parent()
        .context("DLL has no parent directory")?
        .to_owned();
    let config_path = directory.join("SSSniffer.json");
    let config = Config::read(&config_path)?;
    let port = config.port;
    let protocol = Protocol::load(&config, &directory)?;
    let state = Arc::new(AppState::new(directory, protocol));
    let executable = module_path(None)?;
    let standalone = executable
        .file_name()
        .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case("rundll32.exe"));
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?;
    runtime.block_on(async {
        let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port))
            .await
            .with_context(|| format!("Unable to bind 127.0.0.1:{port}"))?;
        capture::start(state.clone())?;
        if !standalone {
            let hook_state = state.clone();
            tokio::spawn(async move {
                DebugPrintLockln!("[INFO] Waiting for GameAssembly.dll...");
                loop {
                    {
                        if let Ok(base) = unsafe { GetModuleHandleW(w!("GameAssembly.dll")) } {
                            if let Err(error) = unsafe { hooks::install(base) } {
                                DebugPrintLockln!("[ERROR] {error:#}");
                                hook_state.set_hook_error(error.to_string());
                            }
                            break;
                        }
                    }
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
            });
        }
        DebugPrintLockln!("[INFO] SSSniffer listening on http://127.0.0.1:{port}");
        server::serve(listener, state).await
    })
}
