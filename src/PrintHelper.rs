use std::{
    io::Write,
    sync::{Mutex, OnceLock},
};
use windows::{
    Win32::{
        Foundation::HANDLE,
        Storage::FileSystem::{
            CreateFileW, FILE_GENERIC_WRITE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
        },
        System::Console::{AllocConsole, GetConsoleWindow, WriteConsoleW},
    },
    core::w,
};

static CONSOLE: OnceLock<Mutex<Option<usize>>> = OnceLock::new();

pub fn print(text: &str) {
    let output = CONSOLE.get_or_init(|| {
        let handle = unsafe {
            if GetConsoleWindow().0.is_null() {
                let _ = AllocConsole();
            }
            CreateFileW(
                w!("CONOUT$"),
                FILE_GENERIC_WRITE.0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                Default::default(),
                None,
            )
            .ok()
            .map(|handle| handle.0 as usize)
        };
        Mutex::new(handle)
    });
    let handle = output.lock().unwrap_or_else(|error| error.into_inner());
    let text_utf16: Vec<u16> = text.encode_utf16().collect();
    if let Some(handle) = *handle {
        if unsafe { WriteConsoleW(HANDLE(handle as *mut _), &text_utf16, None, None) }.is_ok() {
            return;
        }
    }
    let _ = std::io::stderr().write_all(text.as_bytes());
}

#[macro_export]
macro_rules! DebugPrintLockln {
    ($($argument:tt)*) => { $crate::PrintHelper::print(&format!("{}\n", format_args!($($argument)*))) };
}
