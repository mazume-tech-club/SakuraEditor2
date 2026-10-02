//! %APPDATA%\Sakura2\config.ini（key=value の単純な形式）

use std::path::PathBuf;

use crate::filter::engine::{FilterSpec, PatternSpec};
use crate::util;

pub const DEFAULT_REPO: &str = "mazume-tech-club/SakuraEditor2";

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub vi_mode: bool,
    pub vi_hints: bool,
    pub font_name: String,
    pub font_size: u32,
    pub tab_width: u32,
    pub json_indent: u32,
    pub wrap: bool,
    pub line_numbers: bool,
    pub log_levels: bool,
    pub directwrite: bool,
    pub auto_update: bool,
    pub update_repo: String,
    pub github_token: String,
    pub window: Option<(i32, i32, i32, i32)>,
    pub maximized: bool,
    pub filter: FilterSpec,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            vi_mode: false,
            vi_hints: true,
            font_name: "BIZ UDゴシック".into(),
            font_size: 11,
            tab_width: 4,
            json_indent: 4,
            wrap: false,
            line_numbers: true,
            log_levels: true,
            directwrite: false,
            auto_update: true,
            update_repo: DEFAULT_REPO.into(),
            github_token: String::new(),
            window: None,
            maximized: false,
            filter: FilterSpec::default(),
        }
    }
}

pub fn path() -> PathBuf {
    util::roaming_data_dir().join("config.ini")
}

fn b(v: &str) -> bool {
    matches!(v.trim(), "1" | "true" | "on" | "yes")
}

impl Config {
    pub fn load() -> Config {
        match std::fs::read_to_string(path()) {
            Ok(s) => Config::parse(&s),
            Err(_) => Config::default(),
        }
    }

    pub fn parse(s: &str) -> Config {
        let mut c = Config::default();
        for line in s.lines() {
            let line = line.trim();
            if line.starts_with('#') || line.starts_with(';') {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else { continue };
            let (k, v) = (k.trim(), v.trim());
            match k {
                "vi_mode" => c.vi_mode = b(v),
                "vi_hints" => c.vi_hints = b(v),
                "font_name" => c.font_name = v.to_string(),
                "font_size" => c.font_size = v.parse().unwrap_or(c.font_size).clamp(6, 72),
                "tab_width" => c.tab_width = v.parse().unwrap_or(c.tab_width).clamp(1, 16),
                "json_indent" => c.json_indent = v.parse().unwrap_or(c.json_indent).clamp(0, 8),
                "wrap" => c.wrap = b(v),
                "line_numbers" => c.line_numbers = b(v),
                "log_levels" => c.log_levels = b(v),
                "directwrite" => c.directwrite = b(v),
                "auto_update" => c.auto_update = b(v),
                "update_repo" if !v.is_empty() => c.update_repo = v.to_string(),
                "github_token" => c.github_token = v.to_string(),
                "maximized" => c.maximized = b(v),
                "window" => {
                    let n: Vec<i32> = v.split(',').filter_map(|x| x.trim().parse().ok()).collect();
                    if n.len() == 4 && n[2] > 100 && n[3] > 100 {
                        c.window = Some((n[0], n[1], n[2], n[3]));
                    }
                }
                "filter_line_numbers" => c.filter.line_numbers = b(v),
                // filter=除外(0/1),大小(0/1),正規表現(0/1),色,パターン
                "filter" => {
                    let mut it = v.splitn(5, ',');
                    let flags: Vec<&str> = (0..4).filter_map(|_| it.next()).collect();
                    if let (4, Some(text)) = (flags.len(), it.next()) {
                        c.filter.patterns.push(PatternSpec {
                            exclude: b(flags[0]),
                            case_sensitive: b(flags[1]),
                            regex: b(flags[2]),
                            color: flags[3].parse().unwrap_or(0),
                            text: text.to_string(),
                        });
                    }
                }
                _ => {}
            }
        }
        c
    }

    pub fn serialize(&self) -> String {
        let f = |v: bool| if v { "1" } else { "0" };
        let mut s = String::from("# Sakura2 設定ファイル\n");
        s += &format!("vi_mode={}\nvi_hints={}\n", f(self.vi_mode), f(self.vi_hints));
        s += &format!("font_name={}\nfont_size={}\ntab_width={}\n", self.font_name, self.font_size, self.tab_width);
        s += &format!("json_indent={}\nwrap={}\nline_numbers={}\n", self.json_indent, f(self.wrap), f(self.line_numbers));
        s += &format!("log_levels={}\ndirectwrite={}\n", f(self.log_levels), f(self.directwrite));
        s += &format!("auto_update={}\nupdate_repo={}\ngithub_token={}\n", f(self.auto_update), self.update_repo, self.github_token);
        if let Some((x, y, w, h)) = self.window {
            s += &format!("window={x},{y},{w},{h}\n");
        }
        s += &format!("maximized={}\n", f(self.maximized));
        s += &format!("filter_line_numbers={}\n", f(self.filter.line_numbers));
        for p in self.filter.active() {
            s += &format!(
                "filter={},{},{},{},{}\n",
                f(p.exclude), f(p.case_sensitive), f(p.regex), p.color, p.text.replace(['\r', '\n'], "")
            );
        }
        s
    }

    pub fn save(&self) {
        let p = path();
        if let Some(d) = p.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        let _ = std::fs::write(p, self.serialize());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let mut c = Config::default();
        c.vi_mode = true;
        c.window = Some((10, 20, 800, 600));
        c.filter.patterns.push(PatternSpec { text: "a,b=c".into(), exclude: true, regex: true, color: 3, ..Default::default() });
        let back = Config::parse(&c.serialize());
        assert_eq!(back, c);
    }
}
