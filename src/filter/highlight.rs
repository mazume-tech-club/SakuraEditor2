//! 画面に見えている範囲だけを都度ハイライトする。巨大ログでも一瞬で終わる。

use std::sync::OnceLock;

use regex::bytes::Regex;

use crate::sci::Sci;
use crate::sci_consts::*;
use crate::util::rgb;

pub const IND_FILTER_BASE: usize = 8;
pub const IND_SEARCH: usize = 16;
pub const MARK_ERROR: usize = 20;
pub const MARK_WARN: usize = 21;

/// フィルタ条件ごとの色（0xRRGGBB）
pub const PALETTE: [u32; 8] = [
    0xFF5A5A, 0xFFC107, 0x34C759, 0x40A9FF, 0xAF52DE, 0xFF8A00, 0xFF2D78, 0x00BFA5,
];

pub fn setup(sci: &Sci) {
    for (i, c) in PALETTE.iter().enumerate() {
        let ind = IND_FILTER_BASE + i;
        sci.call(SCI_INDICSETSTYLE, ind, INDIC_ROUNDBOX);
        sci.call(SCI_INDICSETFORE, ind, rgb(*c));
        sci.call(SCI_INDICSETALPHA, ind, 90);
        sci.call(SCI_INDICSETOUTLINEALPHA, ind, 200);
        sci.call(SCI_INDICSETUNDER, ind, 1);
    }
    sci.call(SCI_INDICSETSTYLE, IND_SEARCH, INDIC_ROUNDBOX);
    sci.call(SCI_INDICSETFORE, IND_SEARCH, rgb(0xFF9632));
    sci.call(SCI_INDICSETALPHA, IND_SEARCH, 110);
    sci.call(SCI_INDICSETOUTLINEALPHA, IND_SEARCH, 255);
    sci.call(SCI_INDICSETUNDER, IND_SEARCH, 1);

    sci.call(SCI_MARKERDEFINE, MARK_ERROR, SC_MARK_BACKGROUND);
    sci.call(SCI_MARKERSETBACK, MARK_ERROR, rgb(0xFFE3E3));
    sci.call(SCI_MARKERDEFINE, MARK_WARN, SC_MARK_BACKGROUND);
    sci.call(SCI_MARKERSETBACK, MARK_WARN, rgb(0xFFF4CC));
}

fn error_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(r"(?i)\b(ERROR|ERR|FATAL|CRIT(ICAL)?|SEVERE|EXCEPTION|PANIC|Traceback)\b|例外|エラー|異常").unwrap()
    })
}

fn warn_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"(?i)\b(WARN(ING)?|WRN)\b|警告").unwrap())
}

/// 正規表現のコンパイルを別スレッドで先に済ませておく（起動時に呼ぶ）
pub fn warm_up() {
    std::thread::spawn(|| {
        error_re();
        warn_re();
    });
}

#[derive(Default)]
pub struct Highlighter {
    /// (正規表現, 色インデックス)
    pub patterns: Vec<(Regex, usize)>,
    pub search: Option<Regex>,
    /// ERROR / WARN 行の背景色分け
    pub log_levels: bool,
    last: (usize, usize, usize),
    had_any: bool,
}

impl Highlighter {
    pub fn set_patterns(&mut self, sci: &Sci, pats: Vec<(Regex, usize)>) {
        self.patterns = pats;
        self.clear_all(sci);
        self.refresh(sci, true);
    }

    pub fn set_search(&mut self, sci: &Sci, re: Option<Regex>) {
        self.search = re;
        self.clear_all(sci);
        self.refresh(sci, true);
    }

    pub fn set_log_levels(&mut self, sci: &Sci, on: bool) {
        self.log_levels = on;
        if !on {
            sci.call(SCI_MARKERDELETEALL, MARK_ERROR, 0);
            sci.call(SCI_MARKERDELETEALL, MARK_WARN, 0);
        }
        self.refresh(sci, true);
    }

    fn clear_all(&self, sci: &Sci) {
        let len = sci.len() as isize;
        for ind in IND_FILTER_BASE..=IND_SEARCH {
            sci.call(SCI_SETINDICATORCURRENT, ind, 0);
            sci.call(SCI_INDICATORCLEARRANGE, 0, len);
        }
    }

    fn active(&self) -> bool {
        !self.patterns.is_empty() || self.search.is_some() || self.log_levels
    }

    /// 表示範囲を再計算する。force=false の場合は範囲・長さが変わらなければ何もしない。
    pub fn refresh(&mut self, sci: &Sci, force: bool) {
        if sci.len() == 0 {
            return;
        }
        if !self.active() {
            if self.had_any {
                self.clear_all(sci);
                self.had_any = false;
            }
            return;
        }
        // 行の非表示（絞り込み）や折り返しがあっても、実際に見えている行だけを処理する
        let first_vis = sci.call(SCI_GETFIRSTVISIBLELINE, 0, 0) as usize;
        let n_vis = sci.lines_on_screen() + 2;
        let last_line = sci.line_count().saturating_sub(1);
        let mut lines: Vec<usize> = Vec::with_capacity(n_vis);
        for v in first_vis..first_vis + n_vis {
            let l = (sci.call(SCI_DOCLINEFROMVISIBLE, v, 0) as usize).min(last_line);
            if lines.last() != Some(&l) {
                lines.push(l);
            }
        }
        let key = (lines.first().copied().unwrap_or(0), lines.last().copied().unwrap_or(0), sci.len());
        if !force && key == self.last {
            return;
        }
        self.last = key;
        self.had_any = true;
        for &line in &lines {
            let (start, end) = (sci.line_start(line), sci.line_end(line));
            if end < start {
                continue;
            }
            for ind in IND_FILTER_BASE..=IND_SEARCH {
                sci.call(SCI_SETINDICATORCURRENT, ind, 0);
                sci.call(SCI_INDICATORCLEARRANGE, start, (end - start) as isize);
            }
            let text = sci.range_bytes(start, end).to_vec();
            let fill = |ind: usize, re: &Regex| {
                sci.call(SCI_SETINDICATORCURRENT, ind, 0);
                for m in re.find_iter(&text).take(2_000) {
                    if m.end() > m.start() {
                        sci.call(SCI_INDICATORFILLRANGE, start + m.start(), (m.end() - m.start()) as isize);
                    }
                }
            };
            for (re, color) in &self.patterns {
                fill(IND_FILTER_BASE + color % PALETTE.len(), re);
            }
            if let Some(re) = &self.search {
                fill(IND_SEARCH, re);
            }
            if self.log_levels {
                let (err, warn) = if error_re().is_match(&text) { (true, false) } else { (false, warn_re().is_match(&text)) };
                set_marker(sci, line, MARK_ERROR, err);
                set_marker(sci, line, MARK_WARN, warn);
            }
        }
    }
}

fn set_marker(sci: &Sci, line: usize, mark: usize, on: bool) {
    let has = sci.call(SCI_MARKERGET, line, 0) & (1 << mark) != 0;
    if on && !has {
        sci.call(SCI_MARKERADD, line, mark as isize);
    } else if !on && has {
        sci.call(SCI_MARKERDELETE, line, mark as isize);
    }
}
