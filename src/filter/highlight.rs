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
    last: (usize, usize, usize, usize),
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
        // 長い行は横スクロール・折り返し内のスクロールでも行番号が変わらないので、画面左上の位置もキーに含める
        let view = visible_span(sci);
        let key = (lines.first().copied().unwrap_or(0), lines.last().copied().unwrap_or(0), sci.len(), view.0);
        if !force && key == self.last {
            return;
        }
        self.last = key;
        self.had_any = true;
        for &line in &lines {
            let (mut start, mut end) = (sci.line_start(line), sci.line_end(line));
            if end < start {
                continue;
            }
            if end - start > LONG_LINE {
                (start, end) = clip_long_line(sci, start, end, view);
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

/// これを超える長さの行は、画面に見えている前後だけをハイライトする
const LONG_LINE: usize = 16 * 1024;
/// 見えている範囲の前後に余分に処理する文字数（小さなスクロールで色が欠けないように）
const LONG_LINE_MARGIN: isize = 2048;

/// 画面左上と右下の文書位置
fn visible_span(sci: &Sci) -> (usize, usize) {
    let mut rc = windows_sys::Win32::Foundation::RECT { left: 0, top: 0, right: 0, bottom: 0 };
    unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetClientRect(sci.hwnd, &mut rc) };
    let a = sci.call(SCI_POSITIONFROMPOINT, 0, 0) as usize;
    let b = sci.call(SCI_POSITIONFROMPOINT, rc.right.max(0) as usize, rc.bottom.max(0) as isize) as usize;
    (a.min(b), a.max(b))
}

/// 長い行のうち、画面に見えている部分（＋前後の余白）だけを返す
fn clip_long_line(sci: &Sci, start: usize, end: usize, view: (usize, usize)) -> (usize, usize) {
    let wrap = sci.call(SCI_GETWRAPMODE, 0, 0) != 0;
    let (lo, hi) = if wrap {
        view
    } else {
        // 折り返しなし: この行の画面上の y で左端と右端の位置を取る
        let mut rc = windows_sys::Win32::Foundation::RECT { left: 0, top: 0, right: 0, bottom: 0 };
        unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetClientRect(sci.hwnd, &mut rc) };
        let y = sci.call(SCI_POINTYFROMPOSITION, 0, start as isize);
        let a = sci.call(SCI_POSITIONFROMPOINT, 0, y) as usize;
        let b = sci.call(SCI_POSITIONFROMPOINT, rc.right.max(0) as usize, y) as usize;
        (a, b)
    };
    let lo = (sci.call(SCI_POSITIONRELATIVE, lo.clamp(start, end), -LONG_LINE_MARGIN) as usize).max(start);
    let hi = sci.call(SCI_POSITIONRELATIVE, hi.clamp(start, end), LONG_LINE_MARGIN) as usize;
    // 文書末を超えると 0 が返る
    let hi = if hi == 0 { end } else { hi.min(end) };
    (lo, hi.max(lo))
}

fn set_marker(sci: &Sci, line: usize, mark: usize, on: bool) {
    let has = sci.call(SCI_MARKERGET, line, 0) & (1 << mark) != 0;
    if on && !has {
        sci.call(SCI_MARKERADD, line, mark as isize);
    } else if !on && has {
        sci.call(SCI_MARKERDELETE, line, mark as isize);
    }
}
