//! GitHub Releases からのバックグラウンド自動更新。
//! 通信は OS 標準の WinHTTP を使う（社内プロキシ設定をそのまま利用でき、TLS ライブラリも不要）。

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use windows_sys::Win32::Networking::WinHttp::*;
use windows_sys::Win32::Security::Cryptography::{BCRYPT_SHA256_ALG_HANDLE, BCryptHash};

use crate::util::{self, w};

pub const ASSET_EXE: &str = "sakura2.exe";
pub const ASSET_SHA: &str = "sakura2.exe.sha256";
const CHECK_INTERVAL: u64 = 24 * 3600;

pub enum Outcome {
    /// 更新を適用した（次回起動から新バージョン）
    Updated(String),
    UpToDate,
    Skipped,
    Failed(String),
}

pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// "v1.2.10" → [1, 2, 10]
pub fn parse_version(s: &str) -> Vec<u64> {
    let s = s.trim().trim_start_matches(['v', 'V']);
    let core = s.split(['-', '+']).next().unwrap_or("");
    core.split('.').map(|p| p.parse().unwrap_or(0)).collect()
}

pub fn is_newer(remote: &str, local: &str) -> bool {
    let (mut r, mut l) = (parse_version(remote), parse_version(local));
    let n = r.len().max(l.len());
    r.resize(n, 0);
    l.resize(n, 0);
    r > l
}

fn log(msg: &str) {
    let dir = util::local_data_dir();
    let _ = std::fs::create_dir_all(&dir);
    let line = format!("{} {msg}\n", util::timestamp());
    let _ = crate::filter::output::append(&dir.join("update.log"), line.as_bytes());
}

fn state_path() -> PathBuf {
    util::local_data_dir().join("update_state.txt")
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn due() -> bool {
    let last: u64 = std::fs::read_to_string(state_path()).ok().and_then(|s| s.trim().parse().ok()).unwrap_or(0);
    now().saturating_sub(last) >= CHECK_INTERVAL
}

fn mark_checked() {
    let _ = std::fs::create_dir_all(util::local_data_dir());
    let _ = std::fs::write(state_path(), now().to_string());
}

/// 前回の更新で退避した古い exe を消す
pub fn cleanup_old_exe() {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::fs::remove_file(old_exe_path(&exe));
    }
}

fn old_exe_path(exe: &Path) -> PathBuf {
    exe.with_extension("old.exe")
}

/// ワーカースレッドから呼ぶ。force=true はメニューからの手動確認。
pub fn check(repo: &str, token: &str, force: bool) -> Outcome {
    if !force {
        std::thread::sleep(Duration::from_secs(5));
        if !due() {
            return Outcome::Skipped;
        }
    }
    let r = check_inner(repo, token);
    mark_checked();
    match &r {
        Outcome::Updated(v) => log(&format!("updated to {v}")),
        Outcome::Failed(e) => log(&format!("failed: {e}")),
        _ => {}
    }
    r
}

fn check_inner(repo: &str, token: &str) -> Outcome {
    let api = format!("https://api.github.com/repos/{repo}/releases/latest");
    let body = match get(&api, token, "application/vnd.github+json") {
        Ok(b) => b,
        Err(e) => return Outcome::Failed(e),
    };
    let v: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => return Outcome::Failed(format!("JSON: {e}")),
    };
    let tag = v["tag_name"].as_str().unwrap_or_default().to_string();
    if tag.is_empty() {
        return Outcome::Failed("tag_name がありません".into());
    }
    if !is_newer(&tag, current_version()) {
        return Outcome::UpToDate;
    }
    let assets = v["assets"].as_array().cloned().unwrap_or_default();
    let find = |name: &str| -> Option<String> {
        assets.iter().find(|a| a["name"].as_str() == Some(name)).and_then(|a| {
            // トークンがある（プライベートリポジトリ）なら API 経由、無ければ公開 URL
            if token.is_empty() { a["browser_download_url"].as_str() } else { a["url"].as_str() }.map(String::from)
        })
    };
    let (Some(exe_url), Some(sha_url)) = (find(ASSET_EXE), find(ASSET_SHA)) else {
        return Outcome::Failed(format!("{tag} に {ASSET_EXE} / {ASSET_SHA} がありません"));
    };
    let exe = match get(&exe_url, token, "application/octet-stream") {
        Ok(b) => b,
        Err(e) => return Outcome::Failed(e),
    };
    let sha = match get(&sha_url, token, "application/octet-stream") {
        Ok(b) => b,
        Err(e) => return Outcome::Failed(e),
    };
    let expected: String = String::from_utf8_lossy(&sha).split_whitespace().next().unwrap_or("").to_ascii_lowercase();
    let actual = sha256_hex(&exe);
    if expected != actual {
        return Outcome::Failed(format!("SHA-256 不一致 expected={expected} actual={actual}"));
    }
    match install(&exe) {
        Ok(()) => Outcome::Updated(tag),
        Err(e) => Outcome::Failed(e),
    }
}

/// 実行中の exe をリネーム退避し、新しい exe を置く（Windows は実行中 exe のリネームが可能）
fn install(data: &[u8]) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let dl_dir = util::local_data_dir().join("update");
    std::fs::create_dir_all(&dl_dir).map_err(|e| e.to_string())?;
    let staged = dl_dir.join(ASSET_EXE);
    std::fs::write(&staged, data).map_err(|e| format!("保存失敗: {e}"))?;
    let old = old_exe_path(&exe);
    let _ = std::fs::remove_file(&old);
    std::fs::rename(&exe, &old).map_err(|e| format!("退避失敗（書込権限が無い場所?）: {e}"))?;
    if let Err(e) = std::fs::copy(&staged, &exe) {
        let _ = std::fs::rename(&old, &exe);
        return Err(format!("配置失敗: {e}"));
    }
    let _ = std::fs::remove_file(&staged);
    Ok(())
}

pub fn sha256_hex(data: &[u8]) -> String {
    let mut out = [0u8; 32];
    unsafe {
        BCryptHash(BCRYPT_SHA256_ALG_HANDLE, std::ptr::null(), 0, data.as_ptr(), data.len() as u32, out.as_mut_ptr(), 32);
    }
    out.iter().map(|b| format!("{b:02x}")).collect()
}

struct Handle(*mut core::ffi::c_void);
impl Drop for Handle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { WinHttpCloseHandle(self.0) };
        }
    }
}

fn split_url(url: &str) -> Option<(String, String)> {
    let rest = url.strip_prefix("https://")?;
    let (host, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    Some((host.to_string(), path.to_string()))
}

/// HTTPS GET。リダイレクトは自前で辿り、Authorization は api.github.com にだけ付ける。
fn get(url: &str, token: &str, accept: &str) -> Result<Vec<u8>, String> {
    let session = Handle(unsafe {
        WinHttpOpen(w("Sakura2-Updater").as_ptr(), WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, std::ptr::null(), std::ptr::null(), 0)
    });
    if session.0.is_null() {
        return Err("WinHttpOpen 失敗".into());
    }
    let mut url = url.to_string();
    for _ in 0..6 {
        let (host, path) = split_url(&url).ok_or_else(|| format!("不正な URL: {url}"))?;
        let conn = Handle(unsafe { WinHttpConnect(session.0, w(&host).as_ptr(), 443, 0) });
        if conn.0.is_null() {
            return Err(format!("接続失敗: {host}"));
        }
        let req = Handle(unsafe {
            WinHttpOpenRequest(
                conn.0, w("GET").as_ptr(), w(&path).as_ptr(), std::ptr::null(), std::ptr::null(),
                std::ptr::null(), WINHTTP_FLAG_SECURE,
            )
        });
        if req.0.is_null() {
            return Err("WinHttpOpenRequest 失敗".into());
        }
        let never = WINHTTP_OPTION_REDIRECT_POLICY_NEVER;
        unsafe {
            WinHttpSetOption(req.0, WINHTTP_OPTION_REDIRECT_POLICY, &never as *const u32 as _, 4);
        }
        let mut headers = format!("Accept: {accept}\r\nX-GitHub-Api-Version: 2022-11-28\r\n");
        if !token.is_empty() && host == "api.github.com" {
            headers += &format!("Authorization: Bearer {token}\r\n");
        }
        let hw: Vec<u16> = headers.encode_utf16().collect();
        let ok = unsafe { WinHttpSendRequest(req.0, hw.as_ptr(), hw.len() as u32, std::ptr::null(), 0, 0, 0) };
        if ok == 0 || unsafe { WinHttpReceiveResponse(req.0, std::ptr::null_mut()) } == 0 {
            return Err(format!("通信失敗: {host}"));
        }
        let status = query_status(req.0);
        if (300..400).contains(&status) {
            url = query_location(req.0).ok_or("Location ヘッダがありません")?;
            continue;
        }
        if status != 200 {
            return Err(format!("HTTP {status}: {url}"));
        }
        return read_body(req.0);
    }
    Err("リダイレクトが多すぎます".into())
}

fn query_status(req: *mut core::ffi::c_void) -> u32 {
    let mut status = 0u32;
    let mut len = 4u32;
    unsafe {
        WinHttpQueryHeaders(
            req, WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER, std::ptr::null(),
            &mut status as *mut u32 as _, &mut len, std::ptr::null_mut(),
        );
    }
    status
}

fn query_location(req: *mut core::ffi::c_void) -> Option<String> {
    let mut buf = vec![0u16; 4096];
    let mut len = (buf.len() * 2) as u32;
    let ok = unsafe {
        WinHttpQueryHeaders(req, WINHTTP_QUERY_LOCATION, std::ptr::null(), buf.as_mut_ptr() as _, &mut len, std::ptr::null_mut())
    };
    (ok != 0).then(|| util::from_wide(&buf[..len as usize / 2]))
}

fn read_body(req: *mut core::ffi::c_void) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let mut n = 0u32;
        let ok = unsafe { WinHttpReadData(req, buf.as_mut_ptr() as _, buf.len() as u32, &mut n) };
        if ok == 0 {
            return Err("受信失敗".into());
        }
        if n == 0 {
            return Ok(out);
        }
        out.extend_from_slice(&buf[..n as usize]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_compare() {
        assert!(is_newer("v0.2.0", "0.1.9"));
        assert!(is_newer("v1.10.0", "1.9.9"));
        assert!(!is_newer("v1.0.0", "1.0.0"));
        assert!(!is_newer("1.0", "1.0.0"));
        assert!(!is_newer("v0.9.0-beta", "1.0.0"));
    }

    #[test]
    fn sha256_known_value() {
        assert_eq!(sha256_hex(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }

    #[test]
    fn url_split() {
        assert_eq!(split_url("https://api.github.com/repos/a/b"), Some(("api.github.com".into(), "/repos/a/b".into())));
    }
}
