//! Win32 まわりの小物ユーティリティ

use std::path::{Path, PathBuf};

use windows_sys::Win32::Foundation::{HWND, SYSTEMTIME};
use windows_sys::Win32::System::SystemInformation::{GetLocalTime, GetWindowsDirectoryW};
use windows_sys::Win32::UI::WindowsAndMessaging::{GetWindowTextLengthW, GetWindowTextW, SetWindowTextW};

/// UTF-16 + NUL 終端の文字列を作る
pub fn w(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn wpath(p: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    p.as_os_str().encode_wide().chain(std::iter::once(0)).collect()
}

pub fn from_wide(buf: &[u16]) -> String {
    let n = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..n])
}

pub fn get_window_text(h: HWND) -> String {
    unsafe {
        let n = GetWindowTextLengthW(h);
        if n <= 0 {
            return String::new();
        }
        let mut buf = vec![0u16; n as usize + 1];
        GetWindowTextW(h, buf.as_mut_ptr(), buf.len() as i32);
        from_wide(&buf)
    }
}

pub fn set_window_text(h: HWND, s: &str) {
    unsafe {
        SetWindowTextW(h, w(s).as_ptr());
    }
}

/// C:\Windows\Temp（書き込めない環境では %TEMP%）
pub fn windows_temp_dir() -> PathBuf {
    let mut buf = [0u16; 260];
    let n = unsafe { GetWindowsDirectoryW(buf.as_mut_ptr(), buf.len() as u32) };
    if n > 0 {
        let p = PathBuf::from(from_wide(&buf)).join("Temp");
        if dir_writable(&p) {
            return p;
        }
    }
    std::env::temp_dir()
}

fn dir_writable(p: &Path) -> bool {
    let probe = p.join(format!("sakura2_probe_{}.tmp", std::process::id()));
    match std::fs::File::create(&probe) {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

/// %LOCALAPPDATA%\Sakura2
pub fn local_data_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    base.join("Sakura2")
}

/// %APPDATA%\Sakura2
pub fn roaming_data_dir() -> PathBuf {
    let base = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    base.join("Sakura2")
}

/// yyyyMMdd_HHmmss
pub fn timestamp() -> String {
    let mut t: SYSTEMTIME = unsafe { std::mem::zeroed() };
    unsafe { GetLocalTime(&mut t) };
    format!(
        "{:04}{:02}{:02}_{:02}{:02}{:02}",
        t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond
    )
}

/// ファイル名に使えない文字を置換
pub fn sanitize_file_stem(s: &str) -> String {
    s.chars()
        .map(|c| if "\\/:*?\"<>| ".contains(c) { '_' } else { c })
        .take(60)
        .collect()
}

#[inline]
pub fn loword(v: usize) -> u16 {
    (v & 0xffff) as u16
}

#[inline]
pub fn hiword(v: usize) -> u16 {
    ((v >> 16) & 0xffff) as u16
}

/// 0xRRGGBB を Win32 COLORREF (0x00BBGGRR) へ
#[inline]
pub const fn rgb(c: u32) -> isize {
    let r = (c >> 16) & 0xff;
    let g = (c >> 8) & 0xff;
    let b = c & 0xff;
    ((b << 16) | (g << 8) | r) as isize
}
