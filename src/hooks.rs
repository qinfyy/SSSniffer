use crate::{DebugPrintLockln, Util::Scan, capture};
use anyhow::{Result, bail, ensure};
use detours_rs::{
    DetourAttach, DetourDetach, DetourTransactionAbort, DetourTransactionBegin,
    DetourTransactionCommit, DetourUpdateThread,
};
use std::{
    ffi::c_void,
    panic::catch_unwind,
    sync::atomic::{AtomicBool, Ordering},
};
use windows::Win32::{Foundation::HMODULE, System::Threading::GetCurrentThread};

#[repr(C)]
struct ByteArray {
    klass: *mut c_void,
    monitor: *mut c_void,
    bounds: *mut c_void,
    length: usize,
    data: [u8; 0],
}

#[repr(C)]
struct HttpNetMsg {
    klass: *mut c_void,
    monitor: *mut c_void,
    msg_id: i16,
    body: *mut ByteArray,
    callback: *mut c_void,
    receive_msg_id: i16,
}

const _: () = assert!(std::mem::size_of::<HttpNetMsg>() == 0x30);
const _: () = assert!(std::mem::offset_of!(HttpNetMsg, body) == 0x18);
const _: () = assert!(std::mem::offset_of!(ByteArray, data) == 0x20);

type ReadMessage = unsafe extern "system" fn(
    *mut c_void,
    *mut ByteArray,
    i32,
    i32,
    *mut c_void,
) -> *mut HttpNetMsg;
type BuildMessage =
    unsafe extern "system" fn(*mut c_void, *mut HttpNetMsg, *mut c_void, *mut c_void) -> bool;
static mut READ_MESSAGE: *mut c_void = std::ptr::null_mut();
static mut BUILD_MESSAGE: *mut c_void = std::ptr::null_mut();
static INSTALLED: AtomicBool = AtomicBool::new(false);

pub unsafe fn install(module: HMODULE) -> Result<()> {
    ensure!(
        !INSTALLED.load(Ordering::Acquire),
        "Packet hooks are already installed"
    );
    let build = Scan(
        module,
        "4C 89 44 24 ?? 48 89 4C 24 ?? 53 56 57 41 54 41 55 41 56 41 57 48 81 EC ?? ?? ?? ?? 4D 8B F8 4C 8B E2",
    );
    let read = Scan(
        module,
        "48 89 5C 24 ?? 48 89 74 24 ?? 57 41 56 41 57 48 83 EC ?? 45 8B F1 41 8B F8 48 8B F2 80 3D ?? ?? ?? ?? ??",
    );
    ensure!(
        build != 0 && read != 0,
        "Unable to locate packet functions (build=0x{build:X}, read=0x{read:X})"
    );
    BUILD_MESSAGE = build as *mut c_void;
    READ_MESSAGE = read as *mut c_void;
    transaction(|| {
        check(
            DetourAttach(
                &raw mut READ_MESSAGE,
                read_message as *const () as *mut c_void,
            ),
            "Attach readMessage",
        )?;
        check(
            DetourAttach(
                &raw mut BUILD_MESSAGE,
                build_message as *const () as *mut c_void,
            ),
            "Attach BuildMessage",
        )
    })?;
    INSTALLED.store(true, Ordering::Release);
    DebugPrintLockln!("[INFO] Packet hooks installed (build=0x{build:X}, read=0x{read:X})");
    Ok(())
}

pub unsafe fn uninstall() -> Result<()> {
    if !INSTALLED.load(Ordering::Acquire) {
        return Ok(());
    }
    transaction(|| {
        check(
            DetourDetach(
                &raw mut READ_MESSAGE,
                read_message as *const () as *mut c_void,
            ),
            "Detach readMessage",
        )?;
        check(
            DetourDetach(
                &raw mut BUILD_MESSAGE,
                build_message as *const () as *mut c_void,
            ),
            "Detach BuildMessage",
        )
    })?;
    INSTALLED.store(false, Ordering::Release);
    Ok(())
}

unsafe fn transaction(action: impl FnOnce() -> Result<()>) -> Result<()> {
    check(DetourTransactionBegin(), "Begin detour transaction")?;
    let result = check(
        DetourUpdateThread(std::mem::transmute(GetCurrentThread())),
        "Update detour thread",
    )
    .and_then(|_| action());
    if let Err(error) = result {
        DetourTransactionAbort();
        return Err(error);
    }
    check(DetourTransactionCommit(), "Commit detour transaction")
}

fn check(status: i32, operation: &str) -> Result<()> {
    if status != 0 {
        bail!("{operation} failed: Windows error {status}");
    }
    Ok(())
}

unsafe extern "system" fn read_message(
    this: *mut c_void,
    buffer: *mut ByteArray,
    offset: i32,
    length: i32,
    method: *mut c_void,
) -> *mut HttpNetMsg {
    let original: ReadMessage = std::mem::transmute(READ_MESSAGE);
    let result = original(this, buffer, offset, length, method);
    let _ = catch_unwind(|| snapshot(result, true));
    result
}

unsafe extern "system" fn build_message(
    this: *mut c_void,
    message: *mut HttpNetMsg,
    data: *mut c_void,
    method: *mut c_void,
) -> bool {
    let _ = catch_unwind(|| snapshot(message, false));
    let original: BuildMessage = std::mem::transmute(BUILD_MESSAGE);
    original(this, message, data, method)
}

unsafe fn snapshot(message: *mut HttpNetMsg, from_server: bool) {
    if message.is_null() {
        return;
    }
    if (*message).body.is_null() {
        capture::enqueue((*message).msg_id, from_server, &[]);
        return;
    }
    let body = &*(*message).body;
    if body.length > 64 * 1024 * 1024 {
        return;
    }
    let mut payload = std::slice::from_raw_parts(body.data.as_ptr(), body.length);
    if from_server
        && payload.len() >= 16
        && payload[payload.len() - 16..].iter().all(|byte| *byte == 0)
    {
        payload = &payload[..payload.len() - 16];
    }
    capture::enqueue((*message).msg_id, from_server, payload);
}
