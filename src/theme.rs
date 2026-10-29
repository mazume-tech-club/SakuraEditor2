//! テーマ・カラーパレット管理

use std::path::PathBuf;

use crate::util;

#[derive(Clone, Debug)]
pub struct Palette {
    pub bg: u32,
    pub fg: u32,
    pub comment: u32,
    pub string: u32,
    pub keyword: u32,
    pub type_: u32,
    pub number: u32,
    pub preproc: u32,
    pub prop: u32,
    pub error: u32,
    pub tag: u32,
    pub attr: u32,
    pub func: u32,
    pub link: u32,
    pub regex: u32,
    pub diff_added: u32,
    pub diff_deleted: u32,
    pub diff_pos: u32,
    pub caret_line: u32,
    pub selection: u32,
    pub caret: u32,
    pub margin_bg: u32,
    pub margin_fg: u32,
    pub mark_error: u32,
    pub mark_warn: u32,
    pub search_ind: u32,
    pub ui_bg: u32,
    pub ui_fg: u32,
}

impl Palette {
    pub fn light() -> Self {
        Palette {
            bg: 0xFFFFFF,
            fg: 0x000000,
            comment: 0x008000,
            string: 0xA31515,
            keyword: 0x0000FF,
            type_: 0x2B91AF,
            number: 0x098658,
            preproc: 0x808080,
            prop: 0x0451A5,
            error: 0xE00000,
            tag: 0x800000,
            attr: 0xE50000,
            func: 0x795E26,
            link: 0x0066CC,
            regex: 0x811F3F,
            diff_added: 0x008000,
            diff_deleted: 0xC00000,
            diff_pos: 0x800080,
            caret_line: 0xF2F6FF,
            selection: 0xADD6FF,
            caret: 0x000000,
            margin_bg: 0xF0F0F0,
            margin_fg: 0x808080,
            mark_error: 0xFFE3E3,
            mark_warn: 0xFFF4CC,
            search_ind: 0xFF9632,
            ui_bg: 0xF0F0F0,
            ui_fg: 0x000000,
        }
    }

    pub fn dark() -> Self {
        Palette {
            bg: 0x1e1e1e,
            fg: 0xe0e0e0,
            comment: 0x6A9955,
            string: 0xCE9178,
            keyword: 0x569CD6,
            type_: 0x4EC9B0,
            number: 0xB5CEA8,
            preproc: 0x858585,
            prop: 0x9CDCFE,
            error: 0xF48771,
            tag: 0xD7BA7D,
            attr: 0x9CDCFE,
            func: 0xDCDCAA,
            link: 0x569CD6,
            regex: 0xD16969,
            diff_added: 0x6A9955,
            diff_deleted: 0xC72C48,
            diff_pos: 0x9E7FC2,
            caret_line: 0x2D2D30,
            selection: 0x264F78,
            caret: 0xAEAFAD,
            margin_bg: 0x252526,
            margin_fg: 0x858585,
            mark_error: 0x664444,
            mark_warn: 0x664444,
            search_ind: 0xFF6B35,
            ui_bg: 0x252526,
            ui_fg: 0xe0e0e0,
        }
    }

    /// UI 背景が暗いか（ウィンドウ枠もダーク描画にするかの判定）
    pub fn is_dark(&self) -> bool {
        let c = self.ui_bg;
        let (r, g, b) = ((c >> 16) & 0xff, (c >> 8) & 0xff, c & 0xff);
        (r * 299 + g * 587 + b * 114) / 1000 < 128
    }

    pub fn load(name: &str) -> Self {
        match name {
            "light" => Palette::light(),
            "dark" => Palette::dark(),
            _ => Self::load_custom(name),
        }
    }

    fn load_custom(name: &str) -> Self {
        let mut p = Palette::light();
        let path = themes_dir().join(format!("{}.ini", name));
        if let Ok(content) = std::fs::read_to_string(path) {
            for line in content.lines() {
                let line = line.trim();
                if line.starts_with('#') || line.starts_with(';') || line.is_empty() {
                    continue;
                }
                if let Some((k, v)) = line.split_once('=') {
                    let k = k.trim();
                    let v = v.trim();
                    if let Ok(color) = u32::from_str_radix(v, 16) {
                        match k {
                            "bg" => p.bg = color,
                            "fg" => p.fg = color,
                            "comment" => p.comment = color,
                            "string" => p.string = color,
                            "keyword" => p.keyword = color,
                            "type_" => p.type_ = color,
                            "number" => p.number = color,
                            "preproc" => p.preproc = color,
                            "prop" => p.prop = color,
                            "error" => p.error = color,
                            "tag" => p.tag = color,
                            "attr" => p.attr = color,
                            "func" => p.func = color,
                            "link" => p.link = color,
                            "regex" => p.regex = color,
                            "diff_added" => p.diff_added = color,
                            "diff_deleted" => p.diff_deleted = color,
                            "diff_pos" => p.diff_pos = color,
                            "caret_line" => p.caret_line = color,
                            "selection" => p.selection = color,
                            "caret" => p.caret = color,
                            "margin_bg" => p.margin_bg = color,
                            "margin_fg" => p.margin_fg = color,
                            "mark_error" => p.mark_error = color,
                            "mark_warn" => p.mark_warn = color,
                            "search_ind" => p.search_ind = color,
                            "ui_bg" => p.ui_bg = color,
                            "ui_fg" => p.ui_fg = color,
                            _ => {}
                        }
                    }
                }
            }
        }
        p
    }

    pub fn to_ini(&self) -> String {
        let fmt = |v: u32| format!("{:06X}", v);
        format!(
            "# Sakura2 Theme\nbg={}\nfg={}\ncomment={}\nstring={}\nkeyword={}\ntype_={}\nnumber={}\npreproc={}\nprop={}\nerror={}\ntag={}\nattr={}\nfunc={}\nlink={}\nregex={}\ndiff_added={}\ndiff_deleted={}\ndiff_pos={}\ncaret_line={}\nselection={}\ncaret={}\nmargin_bg={}\nmargin_fg={}\nmark_error={}\nmark_warn={}\nsearch_ind={}\nui_bg={}\nui_fg={}\n",
            fmt(self.bg), fmt(self.fg), fmt(self.comment), fmt(self.string),
            fmt(self.keyword), fmt(self.type_), fmt(self.number), fmt(self.preproc),
            fmt(self.prop), fmt(self.error), fmt(self.tag), fmt(self.attr),
            fmt(self.func), fmt(self.link), fmt(self.regex), fmt(self.diff_added),
            fmt(self.diff_deleted), fmt(self.diff_pos), fmt(self.caret_line),
            fmt(self.selection), fmt(self.caret), fmt(self.margin_bg), fmt(self.margin_fg),
            fmt(self.mark_error), fmt(self.mark_warn), fmt(self.search_ind),
            fmt(self.ui_bg), fmt(self.ui_fg)
        )
    }

    pub fn list_custom() -> Vec<String> {
        let dir = themes_dir();
        if !dir.exists() {
            return vec![];
        }
        let mut names = vec![];
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.path().file_stem().and_then(|n| n.to_str()) {
                    if entry.path().extension().map_or(false, |e| e == "ini") {
                        names.push(name.to_string());
                    }
                }
            }
        }
        names.sort();
        names
    }
}

fn themes_dir() -> PathBuf {
    util::roaming_data_dir().join("themes")
}

pub fn path(name: &str) -> PathBuf {
    themes_dir().join(format!("{}.ini", name))
}

pub fn save_theme(name: &str, palette: &Palette) -> std::io::Result<()> {
    let dir = themes_dir();
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.ini", name));
    std::fs::write(path, palette.to_ini())
}

pub fn open_themes_folder() {
    let dir = themes_dir();
    let _ = std::fs::create_dir_all(&dir);
    let dir_str = dir.to_string_lossy();
    let _ = unsafe {
        use windows_sys::Win32::UI::Shell::ShellExecuteW;
        ShellExecuteW(
            std::ptr::null_mut(),
            crate::util::w("open").as_ptr(),
            crate::util::w(&dir_str).as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            1,
        )
    };
}
