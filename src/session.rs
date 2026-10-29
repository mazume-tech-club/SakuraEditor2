//! 終了時のタブ状態の保存と次回起動時の復元（%LOCALAPPDATA%\Sakura2\session\）

use std::path::PathBuf;

use crate::util;

pub struct Entry {
    pub path: Option<PathBuf>,
    /// 未保存の内容（UTF-8）。保存済みなら None
    pub dirty: Option<Vec<u8>>,
    pub enc: String,
    pub eol: String,
    pub caret: usize,
    pub first_line: usize,
}

pub struct Session {
    pub active: usize,
    pub tabs: Vec<Entry>,
}

fn dir() -> PathBuf {
    util::local_data_dir().join("session")
}

fn index_path() -> PathBuf {
    dir().join("session.txt")
}

fn dirty_path(i: usize) -> PathBuf {
    dir().join(format!("tab{i}.txt"))
}

pub fn save(s: &Session) -> std::io::Result<()> {
    clear();
    let d = dir();
    std::fs::create_dir_all(&d)?;
    let mut out = format!("active={}\n", s.active);
    for (i, t) in s.tabs.iter().enumerate() {
        let dirty = match &t.dirty {
            Some(bytes) => {
                std::fs::write(dirty_path(i), bytes)?;
                dirty_path(i).file_name().unwrap().to_string_lossy().into_owned()
            }
            None => String::new(),
        };
        let path = t.path.as_ref().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
        out += &format!("tab={path}\t{dirty}\t{}\t{}\t{}\t{}\n", t.enc, t.eol, t.caret, t.first_line);
    }
    std::fs::write(index_path(), out)
}

pub fn load() -> Option<Session> {
    let text = std::fs::read_to_string(index_path()).ok()?;
    let mut s = Session { active: 0, tabs: Vec::new() };
    for line in text.lines() {
        if let Some(v) = line.strip_prefix("active=") {
            s.active = v.trim().parse().unwrap_or(0);
        } else if let Some(v) = line.strip_prefix("tab=") {
            let f: Vec<&str> = v.split('\t').collect();
            if f.len() < 6 {
                continue;
            }
            let dirty = if f[1].is_empty() { None } else { std::fs::read(dir().join(f[1])).ok() };
            let path = if f[0].is_empty() { None } else { Some(PathBuf::from(f[0])) };
            if path.is_none() && dirty.is_none() {
                continue;
            }
            s.tabs.push(Entry {
                path,
                dirty,
                enc: f[2].to_string(),
                eol: f[3].to_string(),
                caret: f[4].parse().unwrap_or(0),
                first_line: f[5].parse().unwrap_or(0),
            });
        }
    }
    Some(s)
}

pub fn clear() {
    let _ = std::fs::remove_dir_all(dir());
}
