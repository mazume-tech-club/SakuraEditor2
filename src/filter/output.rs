//! フィルタ結果を C:\Windows\Temp に書き出す

use std::io::Write;
use std::path::{Path, PathBuf};

use crate::util;

pub const PREFIX: &str = "sakura2_filter_";

pub fn output_path(source: Option<&Path>) -> PathBuf {
    let stem = source
        .and_then(|p| p.file_stem())
        .map(|s| util::sanitize_file_stem(&s.to_string_lossy()))
        .unwrap_or_else(|| "untitled".into());
    let dir = util::windows_temp_dir();
    let base = format!("{PREFIX}{stem}_{}", util::timestamp());
    let mut p = dir.join(format!("{base}.log"));
    let mut n = 2;
    while p.exists() {
        p = dir.join(format!("{base}_{n}.log"));
        n += 1;
    }
    p
}

pub fn write(path: &Path, data: &[u8]) -> std::io::Result<()> {
    std::fs::write(path, data)
}

pub fn append(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let mut f = std::fs::OpenOptions::new().append(true).create(true).open(path)?;
    f.write_all(data)
}

/// 7 日より古いフィルタ結果ファイルを掃除する（バックグラウンドで呼ぶ）
pub fn cleanup_old() {
    let dir = util::windows_temp_dir();
    let Ok(rd) = std::fs::read_dir(&dir) else { return };
    let limit = std::time::Duration::from_secs(7 * 24 * 3600);
    for e in rd.flatten() {
        let name = e.file_name();
        if !name.to_string_lossy().starts_with(PREFIX) {
            continue;
        }
        let old = e
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|age| age > limit);
        if old {
            let _ = std::fs::remove_file(e.path());
        }
    }
}
