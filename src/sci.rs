//! Scintilla の薄いラッパ。SendMessage ではなくダイレクト関数で呼び出して高速化する。

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, HMENU, SendMessageW, WS_CHILD, WS_CLIPCHILDREN, WS_EX_CLIENTEDGE,
};

use crate::sci_consts::*;

type SciFnDirect = unsafe extern "C" fn(isize, u32, usize, isize) -> isize;

unsafe extern "C" {
    fn Scintilla_RegisterClasses(h: *mut core::ffi::c_void) -> i32;
    fn sk_create_lexer(name: *const u8) -> *mut core::ffi::c_void;
}

pub fn register_classes(hinst: *mut core::ffi::c_void) {
    unsafe {
        Scintilla_RegisterClasses(hinst);
    }
}

#[derive(Clone, Copy)]
pub struct Sci {
    pub hwnd: HWND,
    f: SciFnDirect,
    p: isize,
}

impl Sci {
    pub fn create(parent: HWND, hinst: *mut core::ffi::c_void, id: usize) -> Sci {
        unsafe {
            let hwnd = CreateWindowExW(
                WS_EX_CLIENTEDGE,
                windows_sys::core::w!("Scintilla"),
                std::ptr::null(),
                WS_CHILD | WS_CLIPCHILDREN,
                0,
                0,
                100,
                100,
                parent,
                id as HMENU,
                hinst,
                std::ptr::null(),
            );
            let f = SendMessageW(hwnd, SCI_GETDIRECTFUNCTION, 0, 0);
            let p = SendMessageW(hwnd, SCI_GETDIRECTPOINTER, 0, 0);
            Sci { hwnd, f: std::mem::transmute::<isize, SciFnDirect>(f), p }
        }
    }

    #[inline]
    pub fn call(&self, msg: u32, w: usize, l: isize) -> isize {
        unsafe { (self.f)(self.p, msg, w, l) }
    }

    #[inline]
    pub fn call_str(&self, msg: u32, w: usize, s: &str) -> isize {
        let mut v = Vec::with_capacity(s.len() + 1);
        v.extend_from_slice(s.as_bytes());
        v.push(0);
        self.call(msg, w, v.as_ptr() as isize)
    }

    pub fn set_lexer(&self, name: &str) {
        let mut n = name.as_bytes().to_vec();
        n.push(0);
        let lexer = unsafe { sk_create_lexer(n.as_ptr()) };
        self.call(SCI_SETILEXER, 0, lexer as isize);
    }

    // ---- 文書全体 ----
    #[inline]
    pub fn len(&self) -> usize {
        self.call(SCI_GETLENGTH, 0, 0) as usize
    }

    /// 文書全体のバイト列を借用する（次の変更まで有効）
    pub fn bytes(&self) -> &[u8] {
        let n = self.len();
        if n == 0 {
            return &[];
        }
        let p = self.call(SCI_GETCHARACTERPOINTER, 0, 0) as *const u8;
        unsafe { std::slice::from_raw_parts(p, n) }
    }

    /// 範囲を借用する（ギャップ移動が起きない範囲指定ポインタ）
    pub fn range_bytes(&self, start: usize, end: usize) -> &[u8] {
        if end <= start {
            return &[];
        }
        let p = self.call(SCI_GETRANGEPOINTER, start, (end - start) as isize) as *const u8;
        if p.is_null() {
            return &[];
        }
        unsafe { std::slice::from_raw_parts(p, end - start) }
    }

    pub fn text_range(&self, start: usize, end: usize) -> String {
        String::from_utf8_lossy(self.range_bytes(start, end)).into_owned()
    }

    /// 大きなテキストを高速に流し込む（Undo 履歴なし）
    pub fn load_bytes(&self, data: &[u8]) {
        self.call(SCI_SETUNDOCOLLECTION, 0, 0);
        self.call(SCI_CLEARALL, 0, 0);
        self.call(SCI_ALLOCATE, data.len() + 1024, 0);
        if !data.is_empty() {
            self.call(SCI_ADDTEXT, data.len(), data.as_ptr() as isize);
        }
        self.call(SCI_SETUNDOCOLLECTION, 1, 0);
        self.call(SCI_EMPTYUNDOBUFFER, 0, 0);
        self.call(SCI_SETSAVEPOINT, 0, 0);
        self.call(SCI_GOTOPOS, 0, 0);
    }

    pub fn append(&self, data: &[u8]) {
        if !data.is_empty() {
            self.call(SCI_APPENDTEXT, data.len(), data.as_ptr() as isize);
        }
    }

    // ---- 行・位置 ----
    #[inline]
    pub fn line_count(&self) -> usize {
        self.call(SCI_GETLINECOUNT, 0, 0) as usize
    }
    #[inline]
    pub fn line_of(&self, pos: usize) -> usize {
        self.call(SCI_LINEFROMPOSITION, pos, 0) as usize
    }
    #[inline]
    pub fn line_start(&self, line: usize) -> usize {
        let p = self.call(SCI_POSITIONFROMLINE, line, 0);
        if p < 0 { self.len() } else { p as usize }
    }
    #[inline]
    pub fn line_end(&self, line: usize) -> usize {
        self.call(SCI_GETLINEENDPOSITION, line, 0) as usize
    }
    #[inline]
    pub fn pos_after(&self, pos: usize) -> usize {
        self.call(SCI_POSITIONAFTER, pos, 0) as usize
    }
    #[inline]
    pub fn pos_before(&self, pos: usize) -> usize {
        self.call(SCI_POSITIONBEFORE, pos, 0) as usize
    }
    #[inline]
    pub fn column(&self, pos: usize) -> usize {
        self.call(SCI_GETCOLUMN, pos, 0) as usize
    }
    #[inline]
    pub fn byte_at(&self, pos: usize) -> u8 {
        self.call(SCI_GETCHARAT, pos, 0) as u8
    }

    // ---- キャレット・選択 ----
    #[inline]
    pub fn caret(&self) -> usize {
        self.call(SCI_GETCURRENTPOS, 0, 0) as usize
    }
    #[inline]
    pub fn anchor(&self) -> usize {
        self.call(SCI_GETANCHOR, 0, 0) as usize
    }
    pub fn goto(&self, pos: usize) {
        self.call(SCI_GOTOPOS, pos, 0);
    }
    pub fn set_sel(&self, anchor: usize, caret: usize) {
        self.call(SCI_SETSEL, anchor, caret as isize);
    }
    pub fn sel_text(&self) -> String {
        let (a, b) = self.sel_range();
        self.text_range(a, b)
    }
    pub fn sel_range(&self) -> (usize, usize) {
        let a = self.call(SCI_GETSELECTIONSTART, 0, 0) as usize;
        let b = self.call(SCI_GETSELECTIONEND, 0, 0) as usize;
        (a, b)
    }
    pub fn goto_line_centered(&self, line: usize) {
        self.call(SCI_ENSUREVISIBLEENFORCEPOLICY, line, 0);
        self.call(SCI_GOTOLINE, line, 0);
        let on_screen = self.call(SCI_LINESONSCREEN, 0, 0) as usize;
        let vis = self.call(SCI_VISIBLEFROMDOCLINE, line, 0) as usize;
        self.call(SCI_SETFIRSTVISIBLELINE, vis.saturating_sub(on_screen / 2), 0);
    }

    // ---- 編集 ----
    pub fn insert(&self, pos: usize, s: &str) {
        self.call_str(SCI_INSERTTEXT, pos, s);
    }
    pub fn delete(&self, start: usize, end: usize) {
        if end > start {
            self.call(SCI_DELETERANGE, start, (end - start) as isize);
        }
    }
    /// [start, end) を置換する
    pub fn replace_range(&self, start: usize, end: usize, s: &str) {
        self.call(SCI_SETTARGETRANGE, start, end as isize);
        self.call(SCI_REPLACETARGET, s.len(), s.as_ptr() as isize);
    }
    pub fn begin_undo(&self) {
        self.call(SCI_BEGINUNDOACTION, 0, 0);
    }
    pub fn end_undo(&self) {
        self.call(SCI_ENDUNDOACTION, 0, 0);
    }
    pub fn is_modified(&self) -> bool {
        self.call(SCI_GETMODIFY, 0, 0) != 0
    }

    // ---- 表示 ----
    pub fn first_visible_doc_line(&self) -> usize {
        let v = self.call(SCI_GETFIRSTVISIBLELINE, 0, 0) as usize;
        self.call(SCI_DOCLINEFROMVISIBLE, v, 0) as usize
    }
    pub fn lines_on_screen(&self) -> usize {
        (self.call(SCI_LINESONSCREEN, 0, 0) as usize).max(1)
    }

    pub fn eol_str(&self) -> &'static str {
        match self.call(SCI_GETEOLMODE, 0, 0) {
            SC_EOL_CRLF => "\r\n",
            SC_EOL_CR => "\r",
            _ => "\n",
        }
    }
}

/// Scintilla.h の SCNotification と同じレイアウト
#[repr(C)]
pub struct SCNotification {
    pub hwnd_from: HWND,
    pub id_from: usize,
    pub code: u32,
    pub position: isize,
    pub ch: i32,
    pub modifiers: i32,
    pub modification_type: i32,
    pub text: *const u8,
    pub length: isize,
    pub lines_added: isize,
    pub message: i32,
    pub wparam: usize,
    pub lparam: isize,
    pub line: isize,
    pub fold_level_now: i32,
    pub fold_level_prev: i32,
    pub margin: i32,
    pub list_type: i32,
    pub x: i32,
    pub y: i32,
    pub token: i32,
    pub annotation_lines_added: isize,
    pub updated: i32,
    pub list_completion_method: i32,
    pub character_source: i32,
}
