//! VSCode 風のログフィルタパネル（エディタ下部にドッキング）

pub mod engine;
pub mod highlight;
pub mod output;

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::Graphics::Gdi::HFONT;
use windows_sys::Win32::UI::Controls::{EM_SETCUEBANNER, EM_SETSEL};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::SetFocus;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BN_CLICKED, BS_AUTOCHECKBOX, BS_PUSHBUTTON, DestroyWindow, EN_CHANGE, ES_AUTOHSCROLL,
    STN_CLICKED, SendMessageW, WS_EX_CLIENTEDGE, WS_TABSTOP,
};

use crate::ui::{SS_CENTER, SS_LEFT, SS_NOTIFY};

use crate::ui;
use crate::util::{self, w};
use engine::{FilterSpec, PatternSpec};

pub const ID_BASE: usize = 3000;
const ID_ADD: usize = 3001;
const ID_RUN: usize = 3002;
const ID_LINENUM: usize = 3003;
const ID_CLOSE: usize = 3004;
const ID_TITLE: usize = 3005;
const ID_COUNT: usize = 3006;
const ID_ROW: usize = 3100;
pub const ID_END: usize = 3999;
const MAX_ROWS: usize = 8;

pub struct Row {
    pub swatch: HWND,
    edit: HWND,
    exclude: HWND,
    case: HWND,
    regex: HWND,
    remove: HWND,
    pub color: usize,
}

pub enum Event {
    None,
    /// 条件が変わった（件数・ハイライトを更新）
    Changed,
    Run,
    Close,
    Relayout,
}

pub struct Panel {
    parent: HWND,
    hinst: *mut core::ffi::c_void,
    font: HFONT,
    pub visible: bool,
    title: HWND,
    add: HWND,
    linenum: HWND,
    run: HWND,
    pub count: HWND,
    close: HWND,
    pub rows: Vec<Row>,
    pub count_is_error: bool,
    /// 起動を速くするため、コントロールは最初に表示するときに作る
    created: bool,
    pending: FilterSpec,
}

impl Panel {
    pub fn new(parent: HWND, hinst: *mut core::ffi::c_void, font: HFONT) -> Panel {
        let null = std::ptr::null_mut();
        Panel {
            parent,
            hinst,
            font,
            visible: false,
            title: null,
            add: null,
            linenum: null,
            run: null,
            count: null,
            close: null,
            rows: Vec::new(),
            count_is_error: false,
            created: false,
            pending: FilterSpec { patterns: vec![PatternSpec { regex: true, ..Default::default() }], line_numbers: false },
        }
    }

    fn ensure(&mut self) {
        if self.created {
            return;
        }
        self.created = true;
        let (parent, hinst, font) = (self.parent, self.hinst, self.font);
        let c = |class: &str, text: &str, style, id| ui::control(parent, hinst, 0, class, text, style, id, font);
        self.title = c("STATIC", "フィルタ  Ctrl+Shift+L", SS_LEFT, ID_TITLE);
        self.add = c("BUTTON", "＋ 条件", BS_PUSHBUTTON as u32 | WS_TABSTOP, ID_ADD);
        self.linenum = c("BUTTON", "行番号を付ける", BS_AUTOCHECKBOX as u32 | WS_TABSTOP, ID_LINENUM);
        self.run = c("BUTTON", "▶ 抽出して Temp に出力 (Enter)", BS_PUSHBUTTON as u32 | WS_TABSTOP, ID_RUN);
        self.count = c("STATIC", "", SS_LEFT, ID_COUNT);
        self.close = c("BUTTON", "×", BS_PUSHBUTTON as u32, ID_CLOSE);
        self.add_row(PatternSpec { regex: true, ..Default::default() });
        let spec = std::mem::take(&mut self.pending);
        self.set_spec(&spec);
    }

    pub fn set_font(&mut self, font: HFONT) {
        self.font = font;
        for h in self.all_hwnds() {
            ui::set_font(h, font);
        }
    }

    fn all_hwnds(&self) -> Vec<HWND> {
        if !self.created {
            return Vec::new();
        }
        let mut v = vec![self.title, self.add, self.linenum, self.run, self.count, self.close];
        for r in &self.rows {
            v.extend([r.swatch, r.edit, r.exclude, r.case, r.regex, r.remove]);
        }
        v
    }

    pub fn add_row(&mut self, spec: PatternSpec) -> bool {
        if self.rows.len() >= MAX_ROWS {
            return false;
        }
        let i = self.rows.len();
        let id = ID_ROW + i * 10;
        let (parent, hinst, font) = (self.parent, self.hinst, self.font);
        let c = |ex, class: &str, text: &str, style, id| ui::control(parent, hinst, ex, class, text, style, id, font);
        let color = (0..highlight::PALETTE.len())
            .find(|c| !self.rows.iter().any(|r| r.color == *c))
            .unwrap_or(i % highlight::PALETTE.len());
        let row = Row {
            swatch: c(0, "STATIC", "●", SS_CENTER | SS_NOTIFY, id),
            edit: c(WS_EX_CLIENTEDGE, "EDIT", &spec.text, ES_AUTOHSCROLL as u32 | WS_TABSTOP, id + 1),
            exclude: c(0, "BUTTON", "除外", BS_AUTOCHECKBOX as u32 | WS_TABSTOP, id + 2),
            case: c(0, "BUTTON", "Aa", BS_AUTOCHECKBOX as u32 | WS_TABSTOP, id + 3),
            regex: c(0, "BUTTON", ".*", BS_AUTOCHECKBOX as u32 | WS_TABSTOP, id + 4),
            remove: c(0, "BUTTON", "－", BS_PUSHBUTTON as u32, id + 5),
            color,
        };
        unsafe {
            SendMessageW(row.edit, EM_SETCUEBANNER, 1, w("正規表現（例: ERROR|WARN, \\btimeout\\b）").as_ptr() as isize);
        }
        ui::set_checked(row.exclude, spec.exclude);
        ui::set_checked(row.case, spec.case_sensitive);
        ui::set_checked(row.regex, spec.regex);
        self.rows.push(row);
        true
    }

    fn remove_row(&mut self, i: usize) {
        if self.rows.len() <= 1 {
            // 最後の 1 行は消さずに空にする
            util::set_window_text(self.rows[0].edit, "");
            return;
        }
        let specs: Vec<(PatternSpec, usize)> = (0..self.rows.len())
            .filter(|&k| k != i)
            .map(|k| (self.row_spec(k), self.rows[k].color))
            .collect();
        // ID を詰め直すため作り直す
        for r in self.rows.drain(..) {
            for h in [r.swatch, r.edit, r.exclude, r.case, r.regex, r.remove] {
                unsafe { DestroyWindow(h) };
            }
        }
        for (s, color) in specs {
            self.add_row(s);
            self.rows.last_mut().unwrap().color = color;
        }
        self.show(self.visible);
    }

    fn row_spec(&self, i: usize) -> PatternSpec {
        let r = &self.rows[i];
        PatternSpec {
            text: util::get_window_text(r.edit),
            exclude: ui::is_checked(r.exclude),
            case_sensitive: ui::is_checked(r.case),
            regex: ui::is_checked(r.regex),
            color: r.color,
        }
    }

    pub fn spec(&self) -> FilterSpec {
        if !self.created {
            return self.pending.clone();
        }
        FilterSpec {
            patterns: (0..self.rows.len()).map(|i| self.row_spec(i)).collect(),
            line_numbers: ui::is_checked(self.linenum),
        }
    }

    pub fn set_spec(&mut self, spec: &FilterSpec) {
        if !self.created {
            self.pending = spec.clone();
            return;
        }
        while self.rows.len() > 1 {
            let n = self.rows.len() - 1;
            self.remove_row(n);
        }
        for (i, p) in spec.patterns.iter().enumerate() {
            if i == 0 {
                let r = &self.rows[0];
                util::set_window_text(r.edit, &p.text);
                ui::set_checked(r.exclude, p.exclude);
                ui::set_checked(r.case, p.case_sensitive);
                ui::set_checked(r.regex, p.regex);
            } else {
                self.add_row(p.clone());
            }
        }
        ui::set_checked(self.linenum, spec.line_numbers);
    }

    /// 先頭行に検索語をセットする（選択文字列から開いたとき）
    pub fn set_first_text(&mut self, text: &str) {
        self.ensure();
        util::set_window_text(self.rows[0].edit, text);
    }

    pub fn height(&self, dpi: u32) -> i32 {
        if !self.visible {
            return 0;
        }
        ui::scale(8 + 28 * (self.rows.len() as i32 + 1), dpi)
    }

    pub fn layout(&self, x: i32, y: i32, width: i32, dpi: u32) {
        if !self.visible {
            return;
        }
        let s = |v| ui::scale(v, dpi);
        let rh = s(24);
        let mut cy = y + s(4);
        let mut cx = x + s(6);
        ui::place(self.title, cx, cy + s(4), s(150), rh - s(4));
        cx += s(155);
        ui::place(self.add, cx, cy, s(72), rh);
        cx += s(78);
        ui::place(self.linenum, cx, cy, s(110), rh);
        cx += s(116);
        ui::place(self.run, cx, cy, s(220), rh);
        cx += s(228);
        ui::place(self.close, x + width - s(30), cy, s(24), rh);
        ui::place(self.count, cx, cy + s(4), x + width - s(36) - cx, rh - s(4));
        for r in &self.rows {
            cy += s(28);
            let right = x + width - s(6);
            ui::place(r.swatch, x + s(6), cy + s(3), s(20), rh - s(4));
            let ew = right - s(200) - (x + s(30));
            ui::place(r.edit, x + s(30), cy, ew, rh);
            let mut bx = x + s(30) + ew + s(8);
            ui::place(r.exclude, bx, cy, s(56), rh);
            bx += s(58);
            ui::place(r.case, bx, cy, s(44), rh);
            bx += s(46);
            ui::place(r.regex, bx, cy, s(40), rh);
            ui::place(r.remove, right - s(26), cy, s(24), rh);
        }
    }

    pub fn show(&mut self, on: bool) {
        if on {
            self.ensure();
        }
        self.visible = on;
        for h in self.all_hwnds() {
            ui::show(h, on);
        }
    }

    pub fn focus(&self) {
        if let Some(r) = self.rows.first() {
            unsafe {
                SetFocus(r.edit);
                SendMessageW(r.edit, EM_SETSEL, 0, -1);
            }
        }
    }

    pub fn owns(&self, h: HWND) -> bool {
        self.all_hwnds().contains(&h)
    }

    pub fn is_edit(&self, h: HWND) -> bool {
        self.rows.iter().any(|r| r.edit == h)
    }

    pub fn swatch_color(&self, h: HWND) -> Option<u32> {
        self.rows.iter().find(|r| r.swatch == h).map(|r| highlight::PALETTE[r.color])
    }

    pub fn set_count(&mut self, text: &str, error: bool) {
        if !self.created {
            return;
        }
        self.count_is_error = error;
        util::set_window_text(self.count, text);
    }

    pub fn on_command(&mut self, id: usize, code: u32) -> Event {
        match id {
            ID_ADD => {
                if self.add_row(PatternSpec { regex: true, ..Default::default() }) {
                    self.show(true);
                    unsafe { SetFocus(self.rows.last().unwrap().edit) };
                }
                Event::Relayout
            }
            ID_RUN => Event::Run,
            ID_CLOSE => Event::Close,
            ID_LINENUM => Event::None,
            _ if (ID_ROW..ID_ROW + MAX_ROWS * 10).contains(&id) => {
                let i = (id - ID_ROW) / 10;
                if i >= self.rows.len() {
                    return Event::None;
                }
                match (id - ID_ROW) % 10 {
                    0 if code == STN_CLICKED as u32 => {
                        // 色をクリックで切り替え
                        self.rows[i].color = (self.rows[i].color + 1) % highlight::PALETTE.len();
                        unsafe {
                            windows_sys::Win32::Graphics::Gdi::InvalidateRect(self.rows[i].swatch, std::ptr::null(), 1);
                        }
                        Event::Changed
                    }
                    1 if code == EN_CHANGE as u32 => Event::Changed,
                    2..=4 if code == BN_CLICKED as u32 => Event::Changed,
                    5 if code == BN_CLICKED as u32 => {
                        self.remove_row(i);
                        Event::Relayout
                    }
                    _ => Event::None,
                }
            }
            _ => Event::None,
        }
    }
}
