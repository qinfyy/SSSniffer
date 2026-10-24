use std::collections::HashMap;
use std::ffi::{CString, c_void};
use std::mem::transmute;
use std::sync::OnceLock;
use windows::Win32::Foundation::MAX_PATH;
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows::Win32::System::SystemInformation::GetSystemDirectoryW;
use windows::core::{BOOL, PCSTR, PCWSTR};

type DWORD = u32;
type UINT = u32;
type LPVOID = *mut c_void;
type LPDWORD = *mut DWORD;
type PUINT = *mut UINT;
type LPSTR = *mut u8;
type LPWSTR = *mut u16;
type LPCSTR = PCSTR;
type LPCWSTR = PCWSTR;
type LPCVOID = *const c_void;

#[allow(non_upper_case_globals)]
static OriginalFuncs: OnceLock<HashMap<&str, usize>> = OnceLock::new();

pub const EXPORT_NAMES: &[&str] = &[
    "GetFileVersionInfoA",
    "GetFileVersionInfoByHandle",
    "GetFileVersionInfoExA",
    "GetFileVersionInfoExW",
    "GetFileVersionInfoSizeA",
    "GetFileVersionInfoSizeExA",
    "GetFileVersionInfoSizeExW",
    "GetFileVersionInfoSizeW",
    "GetFileVersionInfoW",
    "VerFindFileA",
    "VerFindFileW",
    "VerInstallFileA",
    "VerInstallFileW",
    "VerLanguageNameA",
    "VerLanguageNameW",
    "VerQueryValueA",
    "VerQueryValueW",
];

pub unsafe fn LoadVersionDll() {
    if OriginalFuncs.get().is_some() {
        return;
    }
    let mut buffer: [u16; MAX_PATH as usize] = [0; MAX_PATH as usize];
    let len = GetSystemDirectoryW(Some(&mut buffer)) as usize;
    if len == 0 || len >= MAX_PATH as usize {
        return;
    }

    let mut path: Vec<u16> = buffer[..len].to_vec();
    path.extend("\\version.dll\0".encode_utf16());
    let pathPtr = PCWSTR(path.as_ptr());

    let version = match LoadLibraryW(pathPtr) {
        Ok(h) if !h.is_invalid() => h,
        _ => {
            return;
        }
    };

    let mut map = HashMap::new();
    for &name in EXPORT_NAMES.iter() {
        let cstrName = CString::new(name).unwrap();
        let func = match GetProcAddress(version, PCSTR(cstrName.as_ptr() as *const _)) {
            Some(f) => f as usize,
            None => {
                continue;
            }
        };
        map.insert(name, func);
    }

    OriginalFuncs.set(map).ok();
}

macro_rules! define_proxy {
    ($name:ident, $ret:ty, ($($param:ident : $ptype:ty),*)) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "system" fn $name($($param: $ptype),*) -> $ret {
            LoadVersionDll();
            let Some(funcAddr) = OriginalFuncs.get().and_then(|functions| functions.get(stringify!($name))) else {
                return Default::default();
            };
            let func: unsafe extern "system" fn($($ptype),*) -> $ret = transmute(*funcAddr);
            func($($param),*)
        }
    };
}

define_proxy!(GetFileVersionInfoA, BOOL, (f: LPCSTR, h: DWORD, l: DWORD, d: LPVOID));
define_proxy!(GetFileVersionInfoByHandle, BOOL, (a: DWORD, b: DWORD, c: DWORD, d: LPVOID));
define_proxy!(GetFileVersionInfoExA, BOOL, (a: DWORD, b: LPCSTR, c: DWORD, d: DWORD, e: LPVOID));
define_proxy!(GetFileVersionInfoExW, BOOL, (a: DWORD, b: LPCWSTR, c: DWORD, d: DWORD, e: LPVOID));
define_proxy!(GetFileVersionInfoSizeA, DWORD, (a: LPCSTR, b: LPDWORD));
define_proxy!(GetFileVersionInfoSizeExA, DWORD, (a: DWORD, b: LPCSTR, c: LPDWORD));
define_proxy!(GetFileVersionInfoSizeExW, DWORD, (a: DWORD, b: LPCWSTR, c: LPDWORD));
define_proxy!(GetFileVersionInfoSizeW, DWORD, (a: LPCWSTR, b: LPDWORD));
define_proxy!(GetFileVersionInfoW, BOOL, (a: LPCWSTR, b: DWORD, c: DWORD, d: LPVOID));
define_proxy!(VerFindFileA, DWORD, (a: DWORD, b: LPCSTR, c: LPCSTR, d: LPCSTR, e: LPSTR, f: PUINT, g: LPSTR, h: PUINT));
define_proxy!(VerFindFileW, DWORD, (a: DWORD, b: LPCWSTR, c: LPCWSTR, d: LPCWSTR, e: LPWSTR, f: PUINT, g: LPWSTR, h: PUINT));
define_proxy!(VerInstallFileA, DWORD, (a: DWORD, b: LPCSTR, c: LPCSTR, d: LPCSTR, e: LPCSTR, f: LPCSTR, g: LPSTR, h: PUINT));
define_proxy!(VerInstallFileW, DWORD, (a: DWORD, b: LPCWSTR, c: LPCWSTR, d: LPCWSTR, e: LPCWSTR, f: LPCWSTR, g: LPWSTR, h: PUINT));
define_proxy!(VerLanguageNameA, DWORD, (a: DWORD, b: LPSTR, c: DWORD));
define_proxy!(VerLanguageNameW, DWORD, (a: DWORD, b: LPWSTR, c: DWORD));
define_proxy!(VerQueryValueA, BOOL, (a: LPCVOID, b: LPCSTR, c: *mut LPVOID, d: PUINT));
define_proxy!(VerQueryValueW, BOOL, (a: LPCVOID, b: LPCWSTR, c: *mut LPVOID, d: PUINT));
