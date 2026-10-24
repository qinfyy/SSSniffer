use std::ptr::null_mut;
use std::sync::{Mutex, Once, OnceLock};
use windows::Win32::Foundation::{HWND, HANDLE};
use windows::Win32::System::Console::{AllocConsole, GetConsoleWindow, WriteConsoleA, WriteConsoleW};
use windows::core::s;
use windows::Win32::Storage::FileSystem::{CreateFileA, FILE_GENERIC_WRITE, FILE_SHARE_WRITE, OPEN_EXISTING};
use crate::Util::Utf8ToAnsi;

#[allow(non_upper_case_globals)]
static InitializedConsole: Once = Once::new();
#[allow(non_upper_case_globals)]
static OutputLock: OnceLock<Mutex<()>> = OnceLock::new();
#[allow(non_upper_case_globals)]
static mut ConsoleOutputHandler: HANDLE = HANDLE(null_mut());

fn InitializeConsole() {
    InitializedConsole.call_once(|| unsafe {
        if GetConsoleWindow() == HWND(null_mut()) {
            AllocConsole().unwrap();
        }

        ConsoleOutputHandler = CreateFileA(s!("\\\\.\\CONOUT$"), FILE_GENERIC_WRITE.0, FILE_SHARE_WRITE, None, OPEN_EXISTING, Default::default(), None).unwrap();
    });
}

pub unsafe fn DebugPrintAnsi(fmt: &str) {
    InitializeConsole();
    let ansiStr = Utf8ToAnsi(fmt).unwrap();
    let mut written = 0u32;
    WriteConsoleA(ConsoleOutputHandler, &ansiStr, Some(&mut written), None).unwrap();
}

pub unsafe fn DebugPrint(fmt: &str) {
    InitializeConsole();
    let utf16Str: Vec<u16> = fmt.encode_utf16().collect();
    let mut written = 0u32;
    WriteConsoleW(ConsoleOutputHandler, &utf16Str, Some(&mut written), None).unwrap();
}

pub unsafe fn DebugPrintLock(fmt: &str) {
    let lock = OutputLock.get_or_init(|| Mutex::new(()));
    let _guard = lock.lock().unwrap();

    DebugPrint(fmt);
}

#[macro_export]
macro_rules! DebugPrint {
    ($($arg:tt)*) => {{
        unsafe { $crate::PrintHelper::DebugPrint(&format!($($arg)*)) }
    }};
}

#[macro_export]
macro_rules! DebugPrintln {
    () => {
        $crate::DebugPrint!("\n");
    };
    ($fmt:expr) => {
        $crate::DebugPrint!("{}\n", format_args!($fmt));
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::DebugPrint!("{}\n", format_args!($fmt, $($arg)*));
    };
}

#[macro_export]
macro_rules! DebugPrintLock {
    ($($arg:tt)*) => {{
        unsafe { $crate::PrintHelper::DebugPrintLock(&format!($($arg)*)) }
    }};
}

#[macro_export]
macro_rules! DebugPrintLockln {
    () => {
        $crate::DebugPrintLock!("\n");
    };
    ($fmt:expr) => {
        $crate::DebugPrintLock!("{}\n", format_args!($fmt));
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::DebugPrintLock!("{}\n", format_args!($fmt, $($arg)*));
    };
}
