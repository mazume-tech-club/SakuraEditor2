//! エクスプローラーの右クリックメニュー「さくらえでぃた弐 で開く」
//! HKCU に登録するので管理者権限は不要。exe の場所が変わったら起動時に書き直す。

use std::path::Path;

use windows_sys::Win32::System::Registry::{
    HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_SZ, RegDeleteTreeW, RegGetValueW, RegSetKeyValueW,
};

use crate::util::{from_wide, w};

const KEY: &str = r"Software\Classes\*\shell\Sakura2";
const CMD_KEY: &str = r"Software\Classes\*\shell\Sakura2\command";
pub const LABEL: &str = "さくらえでぃた弐 で開く";

fn set(key: &str, name: Option<&str>, value: &str) -> bool {
    let v = w(value);
    let n = name.map(w);
    unsafe {
        RegSetKeyValueW(
            HKEY_CURRENT_USER,
            w(key).as_ptr(),
            n.as_ref().map(|n| n.as_ptr()).unwrap_or(std::ptr::null()),
            REG_SZ,
            v.as_ptr() as _,
            (v.len() * 2) as u32,
        ) == 0
    }
}

fn get(key: &str) -> Option<String> {
    let mut buf = vec![0u16; 1024];
    let mut len = (buf.len() * 2) as u32;
    let r = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER, w(key).as_ptr(), std::ptr::null(), RRF_RT_REG_SZ,
            std::ptr::null_mut(), buf.as_mut_ptr() as _, &mut len,
        )
    };
    (r == 0).then(|| from_wide(&buf))
}

fn command_for(exe: &Path) -> String {
    format!("\"{}\" \"%1\"", exe.display())
}

/// 未登録、または exe の場所や表示名が変わっていれば登録し直す。登録したら true
pub fn ensure(exe: &Path) -> bool {
    if get(CMD_KEY).as_deref() == Some(command_for(exe).as_str()) && get(KEY).as_deref() == Some(LABEL) {
        return false;
    }
    register(exe)
}

pub fn register(exe: &Path) -> bool {
    set(KEY, None, LABEL)
        && set(KEY, Some("Icon"), &format!("\"{}\",0", exe.display()))
        && set(CMD_KEY, None, &command_for(exe))
}

pub fn unregister() {
    unsafe {
        RegDeleteTreeW(HKEY_CURRENT_USER, w(KEY).as_ptr());
    }
}
