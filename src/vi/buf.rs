//! Vi エンジンが操作するバッファの抽象。Scintilla 実装とテスト用の文字列実装がある。

use regex::bytes::Regex;

pub trait Buf {
    fn len(&self) -> usize;
    fn byte(&self, pos: usize) -> u8;
    /// 文書全体のバイト列（検索用）
    fn bytes(&self) -> &[u8];
    fn line_count(&self) -> usize;
    fn line_of(&self, pos: usize) -> usize;
    fn line_start(&self, line: usize) -> usize;
    /// 改行を含まない行末
    fn line_end(&self, line: usize) -> usize;
    fn next_pos(&self, pos: usize) -> usize;
    fn prev_pos(&self, pos: usize) -> usize;
    fn text(&self, a: usize, b: usize) -> String;
    fn caret(&self) -> usize;
    fn anchor(&self) -> usize;
    fn set_caret(&mut self, pos: usize);
    fn set_selection(&mut self, anchor: usize, caret: usize);
    fn insert(&mut self, pos: usize, s: &str);
    fn delete(&mut self, a: usize, b: usize);
    fn replace(&mut self, a: usize, b: usize, s: &str) {
        self.delete(a, b);
        self.insert(a, s);
    }
    fn begin_undo(&mut self);
    fn end_undo(&mut self);
    fn undo(&mut self);
    fn redo(&mut self);
    fn eol(&self) -> &'static str;
    fn indent_unit(&self) -> String {
        "    ".into()
    }
    fn first_visible_line(&self) -> usize {
        0
    }
    fn lines_on_screen(&self) -> usize {
        30
    }
    fn set_first_visible_line(&mut self, _line: usize) {}
    fn clipboard_set(&mut self, _s: &str) {}
    fn clipboard_get(&self) -> Option<String> {
        None
    }

    // ---- 既定実装 ----

    fn char_at(&self, pos: usize) -> char {
        if pos >= self.len() {
            return '\n';
        }
        let b0 = self.byte(pos);
        if b0 < 0x80 {
            return b0 as char;
        }
        let n = if b0 >= 0xF0 { 4 } else if b0 >= 0xE0 { 3 } else { 2 };
        let mut v = [0u8; 4];
        for (i, slot) in v.iter_mut().enumerate().take(n) {
            *slot = self.byte(pos + i);
        }
        std::str::from_utf8(&v[..n]).ok().and_then(|s| s.chars().next()).unwrap_or('\u{FFFD}')
    }

    fn is_eol_at(&self, pos: usize) -> bool {
        pos >= self.len() || matches!(self.byte(pos), b'\n' | b'\r')
    }

    fn first_non_blank(&self, line: usize) -> usize {
        let (mut p, e) = (self.line_start(line), self.line_end(line));
        while p < e && matches!(self.byte(p), b' ' | b'\t') {
            p += 1;
        }
        p
    }

    /// 改行を含む行末（次の行頭）
    fn line_end_incl(&self, line: usize) -> usize {
        if line + 1 < self.line_count() { self.line_start(line + 1) } else { self.len() }
    }

    /// 行頭からの文字数
    fn char_col(&self, pos: usize) -> usize {
        let mut p = self.line_start(self.line_of(pos));
        let mut n = 0;
        while p < pos {
            p = self.next_pos(p);
            n += 1;
        }
        n
    }

    fn pos_at_col(&self, line: usize, col: usize, allow_eol: bool) -> usize {
        let (mut p, e) = (self.line_start(line), self.line_end(line));
        let limit = if allow_eol || e == p { e } else { self.prev_pos(e) };
        let mut n = 0;
        while n < col && p < limit {
            p = self.next_pos(p);
            n += 1;
        }
        p
    }

    fn search(&self, re: &Regex, from: usize, forward: bool) -> Option<(usize, usize, bool)> {
        let hay = self.bytes();
        if forward {
            let start = self.next_pos(from).min(hay.len());
            if let Some(m) = re.find_at(hay, start) {
                return Some((m.start(), m.end(), false));
            }
            re.find(hay).map(|m| (m.start(), m.end(), true))
        } else {
            let mut last = None;
            for m in re.find_iter(&hay[..from.min(hay.len())]) {
                last = Some((m.start(), m.end(), false));
            }
            if last.is_some() {
                return last;
            }
            re.find_iter(hay).last().map(|m| (m.start(), m.end(), true))
        }
    }
}

/// テスト用の単純なバッファ（LF 改行）
#[cfg(test)]
#[derive(Default)]
pub struct StrBuf {
    pub s: String,
    pub caret: usize,
    pub anchor: usize,
    undo: Vec<(String, usize)>,
    redo: Vec<(String, usize)>,
    depth: usize,
    pub clip: Option<String>,
}

#[cfg(test)]
impl StrBuf {
    pub fn new(s: &str) -> StrBuf {
        StrBuf { s: s.into(), ..Default::default() }
    }
    fn starts(&self) -> Vec<usize> {
        let mut v = vec![0];
        for (i, b) in self.s.bytes().enumerate() {
            if b == b'\n' {
                v.push(i + 1);
            }
        }
        v
    }
    fn snapshot(&mut self) {
        if self.depth == 0 {
            self.undo.push((self.s.clone(), self.caret));
            self.redo.clear();
        }
    }
}

#[cfg(test)]
impl Buf for StrBuf {
    fn len(&self) -> usize {
        self.s.len()
    }
    fn byte(&self, pos: usize) -> u8 {
        self.s.as_bytes().get(pos).copied().unwrap_or(0)
    }
    fn bytes(&self) -> &[u8] {
        self.s.as_bytes()
    }
    fn line_count(&self) -> usize {
        self.starts().len()
    }
    fn line_of(&self, pos: usize) -> usize {
        self.starts().iter().rposition(|&s| s <= pos).unwrap_or(0)
    }
    fn line_start(&self, line: usize) -> usize {
        self.starts().get(line).copied().unwrap_or(self.s.len())
    }
    fn line_end(&self, line: usize) -> usize {
        let st = self.starts();
        match st.get(line + 1) {
            Some(&n) => n - 1,
            None => self.s.len(),
        }
    }
    fn next_pos(&self, pos: usize) -> usize {
        if pos >= self.s.len() {
            return self.s.len();
        }
        let mut p = pos + 1;
        while p < self.s.len() && !self.s.is_char_boundary(p) {
            p += 1;
        }
        p
    }
    fn prev_pos(&self, pos: usize) -> usize {
        if pos == 0 {
            return 0;
        }
        let mut p = pos - 1;
        while p > 0 && !self.s.is_char_boundary(p) {
            p -= 1;
        }
        p
    }
    fn text(&self, a: usize, b: usize) -> String {
        self.s[a.min(self.s.len())..b.min(self.s.len())].to_string()
    }
    fn caret(&self) -> usize {
        self.caret
    }
    fn anchor(&self) -> usize {
        self.anchor
    }
    fn set_caret(&mut self, pos: usize) {
        self.caret = pos.min(self.s.len());
        self.anchor = self.caret;
    }
    fn set_selection(&mut self, anchor: usize, caret: usize) {
        self.anchor = anchor;
        self.caret = caret;
    }
    fn insert(&mut self, pos: usize, s: &str) {
        self.snapshot();
        self.s.insert_str(pos, s);
        if self.caret >= pos {
            self.caret += s.len();
            self.anchor = self.caret;
        }
    }
    fn delete(&mut self, a: usize, b: usize) {
        if b <= a {
            return;
        }
        self.snapshot();
        self.s.replace_range(a..b, "");
        if self.caret > a {
            self.caret = if self.caret >= b { self.caret - (b - a) } else { a };
        }
        self.anchor = self.caret;
    }
    fn begin_undo(&mut self) {
        if self.depth == 0 {
            self.undo.push((self.s.clone(), self.caret));
            self.redo.clear();
        }
        self.depth += 1;
    }
    fn end_undo(&mut self) {
        self.depth = self.depth.saturating_sub(1);
        if self.depth == 0 {
            if let Some((s, _)) = self.undo.last() {
                if *s == self.s {
                    self.undo.pop();
                }
            }
        }
    }
    fn undo(&mut self) {
        if let Some((s, c)) = self.undo.pop() {
            self.redo.push((std::mem::replace(&mut self.s, s), self.caret));
            self.set_caret(c);
        }
    }
    fn redo(&mut self) {
        if let Some((s, c)) = self.redo.pop() {
            self.undo.push((std::mem::replace(&mut self.s, s), self.caret));
            self.set_caret(c);
        }
    }
    fn eol(&self) -> &'static str {
        "\n"
    }
    fn clipboard_set(&mut self, s: &str) {
        self.clip = Some(s.into());
    }
    fn clipboard_get(&self) -> Option<String> {
        self.clip.clone()
    }
}
