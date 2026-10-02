//! Vi エンジン用の Scintilla バッファ実装

use windows_sys::Win32::Foundation::HGLOBAL;
use windows_sys::Win32::System::DataExchange::{CloseClipboard, GetClipboardData, OpenClipboard};
use windows_sys::Win32::System::Memory::{GlobalLock, GlobalUnlock};

use crate::sci::Sci;
use crate::sci_consts::*;
use crate::vi::buf::Buf;

const CF_UNICODETEXT: u32 = 13;

pub struct SciBuf<'a>(pub &'a Sci);

pub fn clipboard_text() -> Option<String> {
    unsafe {
        if OpenClipboard(std::ptr::null_mut()) == 0 {
            return None;
        }
        let h = GetClipboardData(CF_UNICODETEXT) as HGLOBAL;
        let mut out = None;
        if !h.is_null() {
            let p = GlobalLock(h) as *const u16;
            if !p.is_null() {
                let mut n = 0;
                while *p.add(n) != 0 {
                    n += 1;
                }
                out = Some(String::from_utf16_lossy(std::slice::from_raw_parts(p, n)));
                GlobalUnlock(h);
            }
        }
        CloseClipboard();
        out
    }
}

impl Buf for SciBuf<'_> {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn byte(&self, pos: usize) -> u8 {
        self.0.byte_at(pos)
    }
    fn bytes(&self) -> &[u8] {
        self.0.bytes()
    }
    fn line_count(&self) -> usize {
        self.0.line_count()
    }
    fn line_of(&self, pos: usize) -> usize {
        self.0.line_of(pos)
    }
    fn line_start(&self, line: usize) -> usize {
        self.0.line_start(line)
    }
    fn line_end(&self, line: usize) -> usize {
        self.0.line_end(line)
    }
    fn next_pos(&self, pos: usize) -> usize {
        self.0.pos_after(pos)
    }
    fn prev_pos(&self, pos: usize) -> usize {
        self.0.pos_before(pos)
    }
    fn text(&self, a: usize, b: usize) -> String {
        self.0.text_range(a, b)
    }
    fn caret(&self) -> usize {
        self.0.caret()
    }
    fn anchor(&self) -> usize {
        self.0.anchor()
    }
    fn set_caret(&mut self, pos: usize) {
        self.0.goto(pos.min(self.0.len()));
        self.0.call(SCI_CHOOSECARETX, 0, 0);
    }
    fn set_selection(&mut self, anchor: usize, caret: usize) {
        self.0.set_sel(anchor, caret);
    }
    fn insert(&mut self, pos: usize, s: &str) {
        self.0.insert(pos, s);
    }
    fn delete(&mut self, a: usize, b: usize) {
        self.0.delete(a, b);
    }
    fn replace(&mut self, a: usize, b: usize, s: &str) {
        self.0.replace_range(a, b, s);
    }
    fn begin_undo(&mut self) {
        self.0.begin_undo();
    }
    fn end_undo(&mut self) {
        self.0.end_undo();
    }
    fn undo(&mut self) {
        self.0.call(SCI_UNDO, 0, 0);
    }
    fn redo(&mut self) {
        self.0.call(SCI_REDO, 0, 0);
    }
    fn eol(&self) -> &'static str {
        self.0.eol_str()
    }
    fn indent_unit(&self) -> String {
        if self.0.call(SCI_GETUSETABS, 0, 0) != 0 {
            "\t".into()
        } else {
            " ".repeat(self.0.call(SCI_GETTABWIDTH, 0, 0).max(1) as usize)
        }
    }
    fn first_visible_line(&self) -> usize {
        self.0.first_visible_doc_line()
    }
    fn lines_on_screen(&self) -> usize {
        self.0.lines_on_screen()
    }
    fn set_first_visible_line(&mut self, line: usize) {
        let v = self.0.call(SCI_VISIBLEFROMDOCLINE, line, 0) as usize;
        self.0.call(SCI_SETFIRSTVISIBLELINE, v, 0);
    }
    fn clipboard_set(&mut self, s: &str) {
        self.0.call(SCI_COPYTEXT, s.len(), s.as_ptr() as isize);
    }
    fn clipboard_get(&self) -> Option<String> {
        clipboard_text().map(|s| {
            // クリップボードは CRLF なので文書の改行コードに合わせる
            let eol = self.0.eol_str();
            if eol == "\r\n" { s } else { s.replace("\r\n", eol) }
        })
    }
}
