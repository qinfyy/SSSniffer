use anyhow::{Result, ensure};
use std::{ffi::OsString, os::windows::ffi::OsStringExt, path::PathBuf};
use windows::{Win32::{
    Foundation::HMODULE, Globalization::{GetACP, MULTI_BYTE_TO_WIDE_CHAR_FLAGS, MultiByteToWideChar, WideCharToMultiByte}, System::{
        Diagnostics::Debug::IMAGE_NT_HEADERS64,
        LibraryLoader::GetModuleFileNameW,
        Memory::{MEM_COMMIT, MEMORY_BASIC_INFORMATION, PAGE_GUARD, PAGE_NOACCESS, VirtualQuery},
        SystemServices::IMAGE_DOS_HEADER,
    },
}, core::PCSTR};

pub fn Utf8ToAnsi(s: &str) -> Option<Vec<u8>> {
    if s.is_empty() {
        return Some(Vec::new());
    }

    let utf16Str: Vec<u16> = s.encode_utf16().collect();
    unsafe {
        let codePage = GetACP();
        let sizeNeeded = WideCharToMultiByte(codePage, 0, &utf16Str, None, PCSTR::null(), None);

        if sizeNeeded <= 0 {
            return None;
        }

        let mut buffer = vec![0u8; sizeNeeded as usize];
        let result = WideCharToMultiByte(codePage, 0, &utf16Str, Some(&mut buffer), PCSTR::null(), None);

        if result <= 0 {
            return None;
        }

        Some(buffer)
    }
}

pub fn AnsiToUtf8(str: &[u8]) -> Option<String> {
    if str.is_empty() {
        return Some(String::new());
    }

    unsafe {
        let codePage = GetACP();
        let sizeNeeded = MultiByteToWideChar(codePage, MULTI_BYTE_TO_WIDE_CHAR_FLAGS(0), str, None);

        if sizeNeeded <= 0 {
            return None;
        }

        let mut buffer = vec![0u16; sizeNeeded as usize];
        let result = MultiByteToWideChar(codePage, MULTI_BYTE_TO_WIDE_CHAR_FLAGS(0), str, Some(&mut buffer));

        if result <= 0 {
            return None;
        }

        String::from_utf16(&buffer).ok()
    }
}

pub fn module_path(module: Option<HMODULE>) -> Result<PathBuf> {
    let mut buffer = vec![0u16; 32768];
    let length = unsafe { GetModuleFileNameW(module, &mut buffer) } as usize;
    ensure!(
        length != 0 && length < buffer.len(),
        "Unable to resolve module path"
    );
    Ok(OsString::from_wide(&buffer[..length]).into())
}

pub unsafe fn Scan(module: HMODULE, pattern: &str) -> usize {
    let base = module.0 as usize;
    if base == 0 {
        return 0;
    }
    let dos = &*(base as *const IMAGE_DOS_HEADER);
    if dos.e_magic != 0x5a4d || dos.e_lfanew < 0 {
        return 0;
    }
    let nt = &*((base + dos.e_lfanew as usize) as *const IMAGE_NT_HEADERS64);
    if nt.Signature != 0x4550 {
        return 0;
    }
    let Some(end) = base.checked_add(nt.OptionalHeader.SizeOfImage as usize) else {
        return 0;
    };
    let pattern: Option<Vec<Option<u8>>> = pattern
        .split_whitespace()
        .map(|token| {
            if token == "?" || token == "??" {
                Some(None)
            } else {
                u8::from_str_radix(token, 16).ok().map(Some)
            }
        })
        .collect();
    let Some(pattern) = pattern.filter(|pattern| !pattern.is_empty()) else {
        return 0;
    };
    let mut position = base;
    while position < end {
        let mut region = MEMORY_BASIC_INFORMATION::default();
        if VirtualQuery(
            Some(position as *const _),
            &mut region,
            size_of::<MEMORY_BASIC_INFORMATION>(),
        ) == 0
        {
            break;
        }
        let Some(next) = (region.BaseAddress as usize).checked_add(region.RegionSize) else {
            break;
        };
        if next <= position {
            break;
        }
        let region_end = next.min(end);
        if region.State == MEM_COMMIT
            && region.Protect.0 & (PAGE_GUARD.0 | PAGE_NOACCESS.0) == 0
            && region_end - position >= pattern.len()
        {
            let bytes = std::slice::from_raw_parts(position as *const u8, region_end - position);
            if let Some(offset) = find_pattern(bytes, &pattern) {
                return position + offset;
            }
        }
        position = next;
    }
    0
}

fn find_pattern(bytes: &[u8], pattern: &[Option<u8>]) -> Option<usize> {
    let mut anchor = (0, 0);
    let mut start = 0;
    for index in 0..=pattern.len() {
        if index == pattern.len() || pattern[index].is_none() {
            if index - start > anchor.1 {
                anchor = (start, index - start);
            }
            start = index + 1;
        }
    }
    let (anchor_start, length) = anchor;
    if length == 0 {
        return None;
    }
    let mut skip = [length; 256];
    for index in 0..length - 1 {
        skip[pattern[anchor_start + index].unwrap() as usize] = length - 1 - index;
    }
    let mut position = 0;
    while position + length <= bytes.len() {
        if (0..length)
            .rev()
            .all(|index| pattern[anchor_start + index] == Some(bytes[position + index]))
        {
            if let Some(start) = position.checked_sub(anchor_start) {
                if start + pattern.len() <= bytes.len()
                    && pattern.iter().enumerate().all(|(index, value)| {
                        value.is_none_or(|value| bytes[start + index] == value)
                    })
                {
                    return Some(start);
                }
            }
            position += 1;
        } else {
            position += skip[bytes[position + length - 1] as usize];
        }
    }
    None
}
