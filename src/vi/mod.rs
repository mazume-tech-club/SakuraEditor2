//! Vi モードのエンジン。キー入力を解釈して `Buf` を操作する。
//! Scintilla に依存しないので、テストでは文字列バッファで検証できる。

pub mod buf;
pub mod ex;
pub mod hints;

use buf::Buf;
use regex::bytes::Regex;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Key {
    Char(char),
    Ctrl(char),
    Esc,
    Enter,
    Backspace,
    Del,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
}

impl Key {
    pub fn label(&self) -> String {
        match self {
            Key::Char(c) => c.to_string(),
            Key::Ctrl(c) => format!("Ctrl+{}", c.to_ascii_uppercase()),
            Key::Esc => "<Esc>".into(),
            Key::Enter => "<Enter>".into(),
            Key::Backspace => "<BS>".into(),
            Key::Del => "<Del>".into(),
            Key::Left => "←".into(),
            Key::Right => "→".into(),
            Key::Up => "↑".into(),
            Key::Down => "↓".into(),
            Key::Home => "<Home>".into(),
            Key::End => "<End>".into(),
        }
    }
}

pub fn keys_label(keys: &[Key]) -> String {
    keys.iter().map(|k| k.label()).collect()
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Normal,
    Insert,
    Visual,
    VisualLine,
}

impl Mode {
    pub fn label(self) -> &'static str {
        match self {
            Mode::Normal => "-- NORMAL --",
            Mode::Insert => "-- INSERT --",
            Mode::Visual => "-- VISUAL --",
            Mode::VisualLine => "-- VISUAL LINE --",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Op {
    Delete,
    Change,
    Yank,
    Indent,
    Outdent,
    Lower,
    Upper,
    Toggle,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Motion {
    Left,
    Right,
    Down,
    Up,
    WordFwd(bool),
    WordEnd(bool),
    WordBack(bool),
    LineStart,
    FirstNonBlank,
    LineEnd,
    GotoLast,
    GotoFirst,
    Find { c: char, forward: bool, till: bool },
    RepeatFind(bool),
    MatchPair,
    ParaFwd,
    ParaBack,
    ScreenTop,
    ScreenMid,
    ScreenBottom,
    SearchNext(bool),
    Star(bool),
    NextLine,
    PrevLine,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum MKind {
    Exclusive,
    Inclusive,
    Linewise,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Obj {
    Word { big: bool, around: bool },
    Quote { q: char, around: bool },
    Pair { open: char, close: char, around: bool },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Target {
    Motion(Motion),
    Line,
    Object(Obj),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum InsPos {
    Before,
    LineStart,
    After,
    LineEnd,
    Below,
    Above,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Act {
    PasteAfter,
    PasteBefore,
    Undo,
    Redo,
    Repeat,
    Join,
    Insert(InsPos),
    Visual,
    VisualLine,
    ToggleCase,
    ReplaceChar(char),
    Cmdline(char),
    SaveQuit,
    QuitForce,
    ScrollCenter,
    ScrollTop,
    ScrollBottom,
    HalfDown,
    HalfUp,
    PageDown,
    PageUp,
    LineDown,
    LineUp,
    // Visual モード専用
    SwapEnds,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum CmdKind {
    Move(Motion),
    Operate(Op, Target),
    Act(Act),
}

#[derive(Clone, Debug)]
pub(crate) struct Cmd {
    pub count: Option<usize>,
    pub kind: CmdKind,
    /// カウントを除いたキー列（. で再生する）
    pub body: Vec<Key>,
}

enum Parsed {
    Incomplete,
    Invalid,
    Done(Cmd),
}

#[derive(Clone, Debug)]
struct Change {
    body: Vec<Key>,
    count: Option<usize>,
    insert: Option<String>,
}

struct InsertRec {
    start: usize,
    count: usize,
    record: bool,
}

/// App に依頼する処理
#[derive(Debug, PartialEq)]
pub enum Effect {
    None,
    /// コマンドライン入力を開く（':' '/' '?'）。2 つ目は初期文字列
    Cmdline(char, String),
    /// App 側で実行する Ex コマンド（ZZ など）
    Ex(String),
    Message(String),
}

pub struct Outcome {
    #[cfg_attr(not(test), allow(dead_code))]
    pub consumed: bool,
    pub effect: Effect,
}

impl Outcome {
    fn eaten(effect: Effect) -> Outcome {
        Outcome { consumed: true, effect }
    }
    fn pass() -> Outcome {
        Outcome { consumed: false, effect: Effect::None }
    }
}

pub struct Vi {
    pub mode: Mode,
    pending: Vec<Key>,
    reg: (String, bool),
    last_change: Option<Change>,
    insert: Option<InsertRec>,
    last_find: Option<(char, bool, bool)>,
    pub last_search: Option<(String, bool)>,
    want_col: Option<usize>,
    v_anchor: usize,
    v_caret: usize,
    shown: (usize, usize),
    pub last_visual_lines: Option<(usize, usize)>,
    /// 学習用ヒント（ステータスバーに表示）
    pub hint: String,
    replaying: bool,
    undo_open: bool,
}

impl Default for Vi {
    fn default() -> Self {
        Vi::new()
    }
}

// ---- 文字種 ----
fn class(c: char, big: bool) -> u8 {
    if c.is_whitespace() {
        0
    } else if big {
        1
    } else if c.is_ascii_alphanumeric() || c == '_' {
        2
    } else if c.is_ascii() {
        1
    } else if ('\u{3041}'..='\u{309F}').contains(&c) {
        3 // ひらがな
    } else if ('\u{30A0}'..='\u{30FF}').contains(&c) || ('\u{FF66}'..='\u{FF9F}').contains(&c) {
        4 // カタカナ
    } else if ('\u{4E00}'..='\u{9FFF}').contains(&c) || ('\u{3400}'..='\u{4DBF}').contains(&c) || c == '々' {
        5 // 漢字
    } else if ('\u{3000}'..='\u{303F}').contains(&c) || ('\u{FF01}'..='\u{FF0F}').contains(&c) {
        6 // 全角記号
    } else {
        7
    }
}

fn cls(b: &dyn Buf, p: usize, big: bool) -> u8 {
    if b.is_eol_at(p) { 0 } else { class(b.char_at(p), big) }
}

fn is_empty_line_at(b: &dyn Buf, p: usize) -> bool {
    let l = b.line_of(p);
    b.line_start(l) == b.line_end(l) && p == b.line_start(l)
}

fn word_fwd(b: &dyn Buf, pos: usize, big: bool) -> usize {
    let len = b.len();
    if pos >= len {
        return len;
    }
    let mut p = pos;
    let c0 = cls(b, p, big);
    if c0 != 0 {
        while p < len && cls(b, p, big) == c0 {
            p = b.next_pos(p);
        }
    }
    loop {
        if p >= len {
            return len;
        }
        if b.is_eol_at(p) {
            let l = b.line_of(p);
            if l + 1 >= b.line_count() {
                return len;
            }
            p = b.line_start(l + 1);
            if is_empty_line_at(b, p) {
                return p;
            }
            continue;
        }
        if cls(b, p, big) == 0 {
            p = b.next_pos(p);
            continue;
        }
        return p;
    }
}

fn word_end(b: &dyn Buf, pos: usize, big: bool) -> usize {
    let len = b.len();
    let mut p = b.next_pos(pos);
    while p < len && cls(b, p, big) == 0 {
        p = b.next_pos(p);
    }
    if p >= len {
        return b.prev_pos(len);
    }
    let c = cls(b, p, big);
    loop {
        let q = b.next_pos(p);
        if q >= len || cls(b, q, big) != c {
            return p;
        }
        p = q;
    }
}

fn word_back(b: &dyn Buf, pos: usize, big: bool) -> usize {
    if pos == 0 {
        return 0;
    }
    let mut p = b.prev_pos(pos);
    while p > 0 {
        if is_empty_line_at(b, p) && b.is_eol_at(p) {
            return p;
        }
        if cls(b, p, big) == 0 {
            p = b.prev_pos(p);
            continue;
        }
        break;
    }
    let c = cls(b, p, big);
    while p > 0 {
        let q = b.prev_pos(p);
        if cls(b, q, big) != c {
            break;
        }
        p = q;
    }
    p
}

fn smartcase_regex(pat: &str) -> Option<Regex> {
    let pat = pat.replace("\\<", "\\b").replace("\\>", "\\b");
    let ci = !pat.chars().any(|c| c.is_uppercase());
    regex::bytes::RegexBuilder::new(&pat).case_insensitive(ci).multi_line(true).build().ok()
}

impl Vi {
    pub fn new() -> Vi {
        Vi {
            mode: Mode::Normal,
            pending: Vec::new(),
            reg: (String::new(), false),
            last_change: None,
            insert: None,
            last_find: None,
            last_search: None,
            want_col: None,
            v_anchor: 0,
            v_caret: 0,
            shown: (0, 0),
            last_visual_lines: None,
            hint: String::new(),
            replaying: false,
            undo_open: false,
        }
    }

    pub fn pending_label(&self) -> String {
        keys_label(&self.pending)
    }

    pub fn reset(&mut self, b: &mut dyn Buf) {
        if self.mode == Mode::Insert {
            self.leave_insert(b);
        }
        if matches!(self.mode, Mode::Visual | Mode::VisualLine) {
            let c = self.v_caret;
            b.set_caret(c);
        }
        self.mode = Mode::Normal;
        self.pending.clear();
    }

    /// キー 1 つを処理する
    pub fn key(&mut self, b: &mut dyn Buf, k: Key) -> Outcome {
        if self.mode == Mode::Insert {
            if matches!(k, Key::Esc | Key::Ctrl('[') | Key::Ctrl('c')) {
                self.leave_insert(b);
                self.hint = "<Esc>: ノーマルモードに戻る".into();
                return Outcome::eaten(Effect::None);
            }
            return Outcome::pass();
        }
        self.sync(b);
        if k == Key::Esc && self.pending.is_empty() && self.mode == Mode::Normal {
            self.hint.clear();
            return Outcome::eaten(Effect::None);
        }
        if k == Key::Esc && !self.pending.is_empty() {
            self.pending.clear();
            self.hint = "入力中のコマンドを取り消しました".into();
            return Outcome::eaten(Effect::None);
        }
        self.pending.push(k);
        let visual = matches!(self.mode, Mode::Visual | Mode::VisualLine);
        let parsed = if visual { parse_visual(&self.pending) } else { parse(&self.pending) };
        match parsed {
            Parsed::Incomplete => {
                self.hint = hints::pending(&self.pending);
                Outcome::eaten(Effect::None)
            }
            Parsed::Invalid => {
                let keys = keys_label(&self.pending);
                self.pending.clear();
                if visual && k == Key::Esc {
                    self.exit_visual(b);
                    return Outcome::eaten(Effect::None);
                }
                self.hint = format!("{keys}: 未対応のコマンドです（Shift+F1 でチートシート）");
                Outcome::eaten(Effect::None)
            }
            Parsed::Done(cmd) => {
                self.pending.clear();
                if !self.replaying {
                    self.hint = hints::describe(&cmd);
                }
                let effect = if visual { self.run_visual(b, cmd) } else { self.run(b, cmd) };
                Outcome::eaten(effect)
            }
        }
    }

    /// マウス操作などで選択が変わっていたら状態を合わせる
    fn sync(&mut self, b: &mut dyn Buf) {
        let (a, c) = (b.anchor(), b.caret());
        match self.mode {
            Mode::Normal if a != c => self.enter_visual_from(b, a, c),
            Mode::Visual | Mode::VisualLine if (a, c) != self.shown => {
                if a == c {
                    self.mode = Mode::Normal;
                } else {
                    self.enter_visual_from(b, a, c);
                }
            }
            _ => {}
        }
    }

    fn enter_visual_from(&mut self, b: &mut dyn Buf, a: usize, c: usize) {
        self.mode = Mode::Visual;
        self.v_anchor = a;
        self.v_caret = if c > a { b.prev_pos(c) } else { c };
        self.shown = (a, c);
    }

    fn set_reg(&mut self, b: &mut dyn Buf, text: String, linewise: bool) {
        b.clipboard_set(&text);
        self.reg = (text, linewise);
    }

    fn get_reg(&self, b: &dyn Buf) -> (String, bool) {
        if let Some(c) = b.clipboard_get() {
            if !c.is_empty() && c != self.reg.0 {
                let lw = c.ends_with('\n');
                return (c, lw);
            }
        }
        self.reg.clone()
    }

    fn clamp(&mut self, b: &mut dyn Buf) {
        let c = b.caret();
        let l = b.line_of(c);
        let (s, e) = (b.line_start(l), b.line_end(l));
        if c >= e && e > s {
            b.set_caret(b.prev_pos(e));
        } else if c > e {
            b.set_caret(e);
        }
    }

    fn begin(&mut self, b: &mut dyn Buf) {
        if !self.undo_open {
            b.begin_undo();
            self.undo_open = true;
        }
    }

    fn end(&mut self, b: &mut dyn Buf) {
        if self.undo_open && self.mode != Mode::Insert {
            b.end_undo();
            self.undo_open = false;
        }
    }

    fn enter_insert(&mut self, b: &mut dyn Buf, count: usize) {
        self.mode = Mode::Insert;
        self.insert = Some(InsertRec { start: b.caret(), count, record: true });
    }

    fn leave_insert(&mut self, b: &mut dyn Buf) {
        let rec = self.insert.take();
        self.mode = Mode::Normal;
        if let Some(r) = rec {
            let c = b.caret();
            let text = if c >= r.start { b.text(r.start, c) } else { String::new() };
            if r.count > 1 && !text.is_empty() {
                let more = text.repeat(r.count - 1);
                b.insert(c, &more);
                b.set_caret(c + more.len());
            }
            if r.record {
                if let Some(ch) = &mut self.last_change {
                    ch.insert = Some(text);
                }
            }
        }
        if self.undo_open {
            b.end_undo();
            self.undo_open = false;
        }
        let c = b.caret();
        let l = b.line_of(c);
        if c > b.line_start(l) {
            b.set_caret(b.prev_pos(c));
        }
        self.clamp(b);
    }

    fn record(&mut self, cmd: &Cmd) {
        if !self.replaying {
            self.last_change = Some(Change { body: cmd.body.clone(), count: cmd.count, insert: None });
        }
    }

    // ---- モーション ----

    fn motion(&mut self, b: &mut dyn Buf, cur: usize, m: Motion, n: usize, count: Option<usize>, op: bool) -> Option<(usize, MKind)> {
        let line = b.line_of(cur);
        let last_line = b.line_count().saturating_sub(1);
        let col_motion = |vi: &mut Vi| vi.want_col = None;
        Some(match m {
            Motion::Left => {
                col_motion(self);
                let ls = b.line_start(line);
                let mut p = cur;
                for _ in 0..n {
                    if p > ls {
                        p = b.prev_pos(p);
                    }
                }
                (p, MKind::Exclusive)
            }
            Motion::Right => {
                col_motion(self);
                let le = b.line_end(line);
                let mut p = cur;
                for _ in 0..n {
                    let q = b.next_pos(p);
                    if q <= le && (op || q < le) {
                        p = q;
                    }
                }
                (p, MKind::Exclusive)
            }
            Motion::Down | Motion::Up => {
                let t = if m == Motion::Down { (line + n).min(last_line) } else { line.saturating_sub(n) };
                if t == line {
                    return None;
                }
                let want = *self.want_col.get_or_insert_with(|| b.char_col(cur));
                (b.pos_at_col(t, want, false), MKind::Linewise)
            }
            Motion::WordFwd(big) => {
                col_motion(self);
                let mut p = cur;
                for _ in 0..n {
                    p = word_fwd(b, p, big);
                }
                (p, MKind::Exclusive)
            }
            Motion::WordEnd(big) => {
                col_motion(self);
                let mut p = cur;
                for _ in 0..n {
                    p = word_end(b, p, big);
                }
                (p, MKind::Inclusive)
            }
            Motion::WordBack(big) => {
                col_motion(self);
                let mut p = cur;
                for _ in 0..n {
                    p = word_back(b, p, big);
                }
                (p, MKind::Exclusive)
            }
            Motion::LineStart => {
                col_motion(self);
                (b.line_start(line), MKind::Exclusive)
            }
            Motion::FirstNonBlank => {
                col_motion(self);
                (b.first_non_blank(line), MKind::Exclusive)
            }
            Motion::LineEnd => {
                self.want_col = Some(usize::MAX);
                let t = (line + n - 1).min(last_line);
                let e = b.line_end(t);
                if b.line_start(t) == e {
                    (e, MKind::Exclusive)
                } else {
                    (b.prev_pos(e), MKind::Inclusive)
                }
            }
            Motion::GotoLast | Motion::GotoFirst => {
                col_motion(self);
                let t = match (m, count) {
                    (_, Some(c)) => (c.max(1) - 1).min(last_line),
                    (Motion::GotoLast, None) => last_line,
                    _ => 0,
                };
                (b.first_non_blank(t), MKind::Linewise)
            }
            Motion::Find { c, forward, till } => {
                col_motion(self);
                self.last_find = Some((c, forward, till));
                (find_char(b, cur, c, forward, till, n, false)?, if forward { MKind::Inclusive } else { MKind::Exclusive })
            }
            Motion::RepeatFind(reverse) => {
                col_motion(self);
                let (c, f, t) = self.last_find?;
                let forward = f != reverse;
                (find_char(b, cur, c, forward, t, n, true)?, if forward { MKind::Inclusive } else { MKind::Exclusive })
            }
            Motion::MatchPair => {
                col_motion(self);
                (match_pair(b, cur)?, MKind::Inclusive)
            }
            Motion::ParaFwd | Motion::ParaBack => {
                col_motion(self);
                let mut l = line;
                for _ in 0..n {
                    if m == Motion::ParaFwd {
                        while l < last_line && b.line_start(l) == b.line_end(l) {
                            l += 1;
                        }
                        while l < last_line && b.line_start(l) != b.line_end(l) {
                            l += 1;
                        }
                    } else {
                        while l > 0 && b.line_start(l) == b.line_end(l) {
                            l -= 1;
                        }
                        while l > 0 && b.line_start(l) != b.line_end(l) {
                            l -= 1;
                        }
                    }
                }
                let p = if m == Motion::ParaFwd && l == last_line && b.line_start(l) != b.line_end(l) {
                    b.line_end(l)
                } else {
                    b.line_start(l)
                };
                (p, MKind::Exclusive)
            }
            Motion::ScreenTop | Motion::ScreenMid | Motion::ScreenBottom => {
                col_motion(self);
                let top = b.first_visible_line();
                let bottom = (top + b.lines_on_screen() - 1).min(last_line);
                let t = match m {
                    Motion::ScreenTop => (top + n - 1).min(bottom),
                    Motion::ScreenBottom => bottom.saturating_sub(n - 1).max(top),
                    _ => (top + bottom) / 2,
                };
                (b.first_non_blank(t), MKind::Linewise)
            }
            Motion::SearchNext(reverse) => {
                col_motion(self);
                let (pat, fwd) = self.last_search.clone()?;
                let re = smartcase_regex(&pat)?;
                let mut p = cur;
                let mut wrapped = false;
                for _ in 0..n {
                    let (s, _, w) = b.search(&re, p, fwd != reverse)?;
                    wrapped |= w;
                    p = s;
                }
                if wrapped && !self.replaying {
                    self.hint = format!("/{pat}: 端に達したので反対側から検索しました");
                }
                (p, MKind::Exclusive)
            }
            Motion::Star(forward) => {
                col_motion(self);
                let (s, e) = obj_word(b, cur, false, false)?;
                let w = b.text(s, e);
                if w.trim().is_empty() {
                    return None;
                }
                let pat = format!("\\b{}\\b", regex::escape(&w));
                self.last_search = Some((pat.clone(), forward));
                let re = regex::bytes::Regex::new(&pat).ok()?;
                let mut p = s;
                for _ in 0..n {
                    p = b.search(&re, p, forward)?.0;
                }
                (p, MKind::Exclusive)
            }
            Motion::NextLine | Motion::PrevLine => {
                col_motion(self);
                let t = if m == Motion::NextLine { (line + n).min(last_line) } else { line.saturating_sub(n) };
                if t == line {
                    return None;
                }
                (b.first_non_blank(t), MKind::Linewise)
            }
        })
    }

    // ---- ノーマルモードの実行 ----

    fn run(&mut self, b: &mut dyn Buf, cmd: Cmd) -> Effect {
        let n = cmd.count.unwrap_or(1).max(1);
        let cur = b.caret();
        match cmd.kind {
            CmdKind::Move(m) => {
                if let Some((p, _)) = self.motion(b, cur, m, n, cmd.count, false) {
                    b.set_caret(p);
                    self.clamp(b);
                }
                Effect::None
            }
            CmdKind::Operate(op, target) => {
                if op != Op::Yank {
                    self.record(&cmd);
                }
                let Some((s, e, lw)) = self.op_range(b, cur, op, target, n, cmd.count) else {
                    return Effect::None;
                };
                self.begin(b);
                self.apply(b, op, s, e, lw, cmd.count.unwrap_or(1));
                self.end(b);
                Effect::None
            }
            CmdKind::Act(a) => self.act(b, a, n, &cmd),
        }
    }

    /// オペレータの対象範囲 (開始, 終了(排他), 行単位か)
    fn op_range(&mut self, b: &mut dyn Buf, cur: usize, op: Op, target: Target, n: usize, count: Option<usize>) -> Option<(usize, usize, bool)> {
        match target {
            Target::Line => {
                let l = b.line_of(cur);
                let last = (l + n - 1).min(b.line_count() - 1);
                Some((b.line_start(l), b.line_start(last), true))
            }
            Target::Object(o) => {
                let (s, e) = match o {
                    Obj::Word { big, around } => obj_word(b, cur, big, around)?,
                    Obj::Quote { q, around } => obj_quote(b, cur, q, around)?,
                    Obj::Pair { open, close, around } => obj_pair(b, cur, open, close, around)?,
                };
                Some((s, e, false))
            }
            Target::Motion(m) => {
                // cw は ce と同じ（Vim の慣習）
                let m = match m {
                    Motion::WordFwd(big) if op == Op::Change && cls(b, cur, big) != 0 => Motion::WordEnd(big),
                    m => m,
                };
                let (p, kind) = self.motion(b, cur, m, n, count, true)?;
                let (mut s, mut e) = if p < cur { (p, cur) } else { (cur, p) };
                match kind {
                    MKind::Linewise => return Some((b.line_start(b.line_of(s)), b.line_start(b.line_of(e)), true)),
                    MKind::Inclusive => e = b.next_pos(e).min(b.line_end(b.line_of(e)).max(e)),
                    MKind::Exclusive => {
                        // 次行の行頭で終わる排他モーションは前の行末までにする（dw を最後の単語で使う場合）
                        let el = b.line_of(e);
                        if e > s && el > b.line_of(s) && e == b.line_start(el) {
                            e = b.line_end(el - 1);
                        }
                    }
                }
                if e > b.len() {
                    e = b.len();
                }
                if s > e {
                    std::mem::swap(&mut s, &mut e);
                }
                Some((s, e, false))
            }
        }
    }

    /// 行単位の場合 s, e はそれぞれ先頭行・最終行の行頭
    fn apply(&mut self, b: &mut dyn Buf, op: Op, s: usize, e: usize, linewise: bool, _count: usize) {
        let eol = b.eol();
        if linewise {
            let (l1, l2) = (b.line_of(s), b.line_of(e));
            let (ls, le) = (b.line_start(l1), b.line_end_incl(l2));
            match op {
                Op::Yank => {
                    let mut t = b.text(ls, le);
                    if !t.ends_with('\n') {
                        t.push_str(eol);
                    }
                    let lines = l2 - l1 + 1;
                    self.set_reg(b, t, true);
                    if !self.replaying && lines > 1 {
                        self.hint = format!("{lines} 行ヤンクしました");
                    }
                }
                Op::Delete => {
                    let mut t = b.text(ls, le);
                    if !t.ends_with('\n') {
                        t.push_str(eol);
                    }
                    self.set_reg(b, t, true);
                    if le == b.len() && l1 > 0 && !b.text(ls, le).ends_with('\n') {
                        // 最終行（改行なし）を消すときは前の行の改行を消す
                        let pe = b.line_end(l1 - 1);
                        b.delete(pe, le);
                        b.set_caret(b.first_non_blank(l1 - 1));
                    } else {
                        b.delete(ls, le);
                        let mut l = b.line_of(ls.min(b.len()));
                        // 末尾改行の後ろの空行に落ちたら 1 行上へ（Vim と同じ見た目にする）
                        if l > 0 && l + 1 == b.line_count() && b.line_start(l) == b.len() {
                            l -= 1;
                        }
                        b.set_caret(b.first_non_blank(l));
                    }
                }
                Op::Change => {
                    let ce = b.line_end(l2);
                    let mut t = b.text(ls, le);
                    if !t.ends_with('\n') {
                        t.push_str(eol);
                    }
                    self.set_reg(b, t, true);
                    let indent = b.text(ls, b.first_non_blank(l1));
                    b.replace(ls, ce, &indent);
                    b.set_caret(ls + indent.len());
                    self.enter_insert(b, 1);
                    return;
                }
                Op::Indent | Op::Outdent => {
                    let unit = b.indent_unit();
                    for l in l1..=l2 {
                        let st = b.line_start(l);
                        if op == Op::Indent {
                            if b.line_end(l) > st {
                                b.insert(st, &unit);
                            }
                        } else {
                            let fnb = b.first_non_blank(l);
                            let ws = b.text(st, fnb);
                            let remove = if ws.starts_with('\t') { 1 } else { ws.chars().take_while(|&c| c == ' ').count().min(unit.len().max(1)) };
                            b.delete(st, st + remove);
                        }
                    }
                    b.set_caret(b.first_non_blank(l1));
                }
                Op::Lower | Op::Upper | Op::Toggle => {
                    let ce = b.line_end(l2);
                    let t = transform(&b.text(ls, ce), op);
                    b.replace(ls, ce, &t);
                    b.set_caret(b.first_non_blank(l1));
                }
            }
            if op != Op::Change {
                self.clamp(b);
            }
            return;
        }
        match op {
            Op::Yank => {
                let t = b.text(s, e);
                self.set_reg(b, t, false);
                b.set_caret(s);
            }
            Op::Delete => {
                let t = b.text(s, e);
                self.set_reg(b, t, false);
                b.delete(s, e);
                b.set_caret(s);
            }
            Op::Change => {
                let t = b.text(s, e);
                self.set_reg(b, t, false);
                b.delete(s, e);
                b.set_caret(s);
                self.enter_insert(b, 1);
                return;
            }
            Op::Indent | Op::Outdent => {
                let (l1, l2) = (b.line_of(s), b.line_of(e));
                let (a, z) = (b.line_start(l1), b.line_start(l2));
                return self.apply(b, op, a, z, true, 1);
            }
            Op::Lower | Op::Upper | Op::Toggle => {
                let t = transform(&b.text(s, e), op);
                b.replace(s, e, &t);
                b.set_caret(s);
            }
        }
        self.clamp(b);
    }

    fn act(&mut self, b: &mut dyn Buf, a: Act, n: usize, cmd: &Cmd) -> Effect {
        let cur = b.caret();
        let line = b.line_of(cur);
        match a {
            Act::PasteAfter | Act::PasteBefore => {
                self.record(cmd);
                let (text, lw) = self.get_reg(b);
                if text.is_empty() {
                    return Effect::Message("レジスタが空です".into());
                }
                self.begin(b);
                let all = text.repeat(n);
                if lw {
                    let at = if a == Act::PasteAfter { b.line_end_incl(line) } else { b.line_start(line) };
                    let first_line = if a == Act::PasteAfter { line + 1 } else { line };
                    if a == Act::PasteAfter && at == b.len() && !b.text(b.line_start(line), at).ends_with('\n') {
                        let eol = b.eol();
                        let body = all.strip_suffix("\r\n").or_else(|| all.strip_suffix('\n')).unwrap_or(&all);
                        b.insert(at, &format!("{eol}{body}"));
                    } else {
                        b.insert(at, &all);
                    }
                    b.set_caret(b.first_non_blank(first_line));
                } else {
                    let le = b.line_end(line);
                    let at = if a == Act::PasteAfter && le > b.line_start(line) { b.next_pos(cur).min(le) } else { cur };
                    b.insert(at, &all);
                    b.set_caret(b.prev_pos(at + all.len()).max(at));
                }
                self.end(b);
                self.clamp(b);
            }
            Act::Undo => {
                for _ in 0..n {
                    b.undo();
                }
                self.clamp(b);
            }
            Act::Redo => {
                for _ in 0..n {
                    b.redo();
                }
                self.clamp(b);
            }
            Act::Repeat => {
                let Some(ch) = self.last_change.clone() else {
                    return Effect::Message("繰り返す変更がありません".into());
                };
                let count = cmd.count.or(ch.count);
                let mut keys: Vec<Key> = count.map(|c| c.to_string().chars().map(Key::Char).collect()).unwrap_or_default();
                keys.extend(ch.body.iter().copied());
                self.replaying = true;
                for k in keys {
                    self.key(b, k);
                }
                if self.mode == Mode::Insert {
                    if let Some(t) = &ch.insert {
                        let c = b.caret();
                        b.insert(c, t);
                        b.set_caret(c + t.len());
                    }
                    self.leave_insert(b);
                }
                self.replaying = false;
                let mut ch2 = ch;
                ch2.count = count;
                self.last_change = Some(ch2);
            }
            Act::Join => {
                self.record(cmd);
                let times = n.max(2) - 1;
                self.begin(b);
                for _ in 0..times {
                    let l = b.line_of(b.caret());
                    if l + 1 >= b.line_count() {
                        break;
                    }
                    let e = b.line_end(l);
                    let next_fnb = b.first_non_blank(l + 1);
                    let next_empty = next_fnb == b.line_end(l + 1);
                    let prev_blank = e == b.line_start(l) || matches!(b.byte(b.prev_pos(e)), b' ' | b'\t');
                    let sep = if next_empty || prev_blank || b.byte(next_fnb) == b')' { "" } else { " " };
                    b.replace(e, next_fnb, sep);
                    b.set_caret(e);
                }
                self.end(b);
            }
            Act::Insert(pos) => {
                self.record(cmd);
                self.begin(b);
                let (ls, le) = (b.line_start(line), b.line_end(line));
                match pos {
                    InsPos::Before => {}
                    InsPos::LineStart => b.set_caret(b.first_non_blank(line)),
                    InsPos::After => {
                        if le > ls {
                            b.set_caret(b.next_pos(cur).min(le));
                        }
                    }
                    InsPos::LineEnd => b.set_caret(le),
                    InsPos::Below | InsPos::Above => {
                        let indent = b.text(ls, b.first_non_blank(line));
                        let eol = b.eol();
                        if pos == InsPos::Below {
                            b.insert(le, &format!("{eol}{indent}"));
                            b.set_caret(le + eol.len() + indent.len());
                        } else {
                            b.insert(ls, &format!("{indent}{eol}"));
                            b.set_caret(ls + indent.len());
                        }
                    }
                }
                self.enter_insert(b, n);
            }
            Act::Visual | Act::VisualLine => {
                self.mode = if a == Act::Visual { Mode::Visual } else { Mode::VisualLine };
                self.v_anchor = cur;
                self.v_caret = cur;
                self.show_visual(b);
            }
            Act::ToggleCase => {
                self.record(cmd);
                let le = b.line_end(line);
                let mut e = cur;
                for _ in 0..n {
                    if e < le {
                        e = b.next_pos(e);
                    }
                }
                self.begin(b);
                let t = transform(&b.text(cur, e), Op::Toggle);
                b.replace(cur, e, &t);
                self.end(b);
                b.set_caret(e);
                self.clamp(b);
            }
            Act::ReplaceChar(c) => {
                let le = b.line_end(line);
                let mut e = cur;
                for _ in 0..n {
                    if e >= le {
                        return Effect::None;
                    }
                    e = b.next_pos(e);
                }
                self.record(cmd);
                self.begin(b);
                let rep: String = std::iter::repeat_n(c, n).collect();
                b.replace(cur, e, &rep);
                self.end(b);
                b.set_caret(b.prev_pos(cur + rep.len()));
            }
            Act::Cmdline(c) => return Effect::Cmdline(c, String::new()),
            Act::SaveQuit => return Effect::Ex("x".into()),
            Act::QuitForce => return Effect::Ex("q!".into()),
            Act::ScrollCenter | Act::ScrollTop | Act::ScrollBottom => {
                let h = b.lines_on_screen();
                let top = match a {
                    Act::ScrollCenter => line.saturating_sub(h / 2),
                    Act::ScrollTop => line,
                    _ => line.saturating_sub(h.saturating_sub(1)),
                };
                b.set_first_visible_line(top);
            }
            Act::HalfDown | Act::HalfUp | Act::PageDown | Act::PageUp | Act::LineDown | Act::LineUp => {
                let h = b.lines_on_screen();
                let d = match a {
                    Act::HalfDown | Act::HalfUp => (h / 2).max(1),
                    Act::PageDown | Act::PageUp => h.saturating_sub(2).max(1),
                    _ => n,
                };
                let down = matches!(a, Act::HalfDown | Act::PageDown | Act::LineDown);
                let last = b.line_count() - 1;
                let top = b.first_visible_line();
                let ntop = if down { (top + d).min(last) } else { top.saturating_sub(d) };
                b.set_first_visible_line(ntop);
                if !matches!(a, Act::LineDown | Act::LineUp) {
                    let t = if down { (line + d).min(last) } else { line.saturating_sub(d) };
                    b.set_caret(b.first_non_blank(t));
                } else if line < ntop || line >= ntop + h {
                    b.set_caret(b.first_non_blank(if down { ntop } else { (ntop + h - 1).min(last) }));
                }
                self.clamp(b);
            }
            Act::SwapEnds => {}
        }
        Effect::None
    }

    // ---- ビジュアルモード ----

    fn show_visual(&mut self, b: &mut dyn Buf) {
        let (a, c) = (self.v_anchor, self.v_caret);
        if self.mode == Mode::VisualLine {
            let (la, lc) = (b.line_of(a), b.line_of(c));
            if lc >= la {
                b.set_selection(b.line_start(la), b.line_end_incl(lc));
            } else {
                b.set_selection(b.line_end_incl(la), b.line_start(lc));
            }
        } else if c >= a {
            b.set_selection(a, b.next_pos(c).min(b.len()).max(c));
        } else {
            b.set_selection(b.next_pos(a), c);
        }
        self.shown = (b.anchor(), b.caret());
    }

    fn exit_visual(&mut self, b: &mut dyn Buf) {
        self.remember_visual(b);
        self.mode = Mode::Normal;
        let c = self.v_caret;
        b.set_caret(c);
        self.clamp(b);
    }

    fn remember_visual(&mut self, b: &dyn Buf) {
        let (la, lc) = (b.line_of(self.v_anchor), b.line_of(self.v_caret));
        self.last_visual_lines = Some((la.min(lc), la.max(lc)));
    }

    fn visual_range(&self, b: &dyn Buf) -> (usize, usize, bool) {
        let (s, e) = if self.v_anchor <= self.v_caret { (self.v_anchor, self.v_caret) } else { (self.v_caret, self.v_anchor) };
        if self.mode == Mode::VisualLine {
            (b.line_start(b.line_of(s)), b.line_start(b.line_of(e)), true)
        } else {
            (s, b.next_pos(e).min(b.len()), false)
        }
    }

    fn run_visual(&mut self, b: &mut dyn Buf, cmd: Cmd) -> Effect {
        let n = cmd.count.unwrap_or(1).max(1);
        match cmd.kind {
            CmdKind::Move(m) => {
                if let Some((p, _)) = self.motion(b, self.v_caret, m, n, cmd.count, false) {
                    self.v_caret = p.min(b.len());
                    let l = b.line_of(self.v_caret);
                    if self.v_caret >= b.line_end(l) && b.line_end(l) > b.line_start(l) {
                        self.v_caret = b.prev_pos(b.line_end(l));
                    }
                }
                self.show_visual(b);
                Effect::None
            }
            CmdKind::Operate(op, _) => {
                self.remember_visual(b);
                let (s, e, lw) = self.visual_range(b);
                self.mode = Mode::Normal;
                b.set_caret(s);
                self.begin(b);
                self.apply(b, op, s, e, lw, 1);
                self.end(b);
                Effect::None
            }
            CmdKind::Act(a) => match a {
                Act::Visual | Act::VisualLine => {
                    let want = if a == Act::Visual { Mode::Visual } else { Mode::VisualLine };
                    if self.mode == want {
                        self.exit_visual(b);
                    } else {
                        self.mode = want;
                        self.show_visual(b);
                    }
                    Effect::None
                }
                Act::SwapEnds => {
                    std::mem::swap(&mut self.v_anchor, &mut self.v_caret);
                    self.show_visual(b);
                    Effect::None
                }
                Act::Cmdline(':') => {
                    self.exit_visual(b);
                    Effect::Cmdline(':', "'<,'>".into())
                }
                Act::Cmdline(c) => {
                    self.exit_visual(b);
                    Effect::Cmdline(c, String::new())
                }
                Act::PasteAfter | Act::PasteBefore => {
                    let (text, _) = self.get_reg(b);
                    let (s, e, lw) = self.visual_range(b);
                    let (s, e) = if lw { (s, b.line_end_incl(b.line_of(e))) } else { (s, e) };
                    self.remember_visual(b);
                    self.mode = Mode::Normal;
                    let old = b.text(s, e);
                    b.begin_undo();
                    b.replace(s, e, &text);
                    b.end_undo();
                    self.set_reg(b, old, lw);
                    b.set_caret(s);
                    self.clamp(b);
                    Effect::None
                }
                Act::Join => {
                    let (s, e, _) = self.visual_range(b);
                    let lines = b.line_of(e) - b.line_of(s) + 1;
                    self.exit_visual(b);
                    b.set_caret(s);
                    let c = Cmd { count: Some(lines.max(2)), kind: CmdKind::Act(Act::Join), body: vec![Key::Char('J')] };
                    self.run(b, c)
                }
                _ => Effect::None,
            },
        }
    }

    /// '/' や '?' の検索を実行する（コマンドラインで Enter が押されたとき）
    pub fn search(&mut self, b: &mut dyn Buf, pat: &str, forward: bool) -> Result<(), String> {
        let pat = if pat.is_empty() {
            match &self.last_search {
                Some((p, _)) => p.clone(),
                None => return Err("検索パターンがありません".into()),
            }
        } else {
            pat.to_string()
        };
        self.last_search = Some((pat.clone(), forward));
        let re = smartcase_regex(&pat).ok_or_else(|| format!("正規表現エラー: {pat}"))?;
        let cur = if matches!(self.mode, Mode::Visual | Mode::VisualLine) { self.v_caret } else { b.caret() };
        match b.search(&re, cur, forward) {
            Some((s, _, wrapped)) => {
                if matches!(self.mode, Mode::Visual | Mode::VisualLine) {
                    self.v_caret = s;
                    self.show_visual(b);
                } else {
                    b.set_caret(s);
                }
                self.hint = if wrapped { format!("/{pat}: 端に達したので反対側から検索しました") } else { format!("/{pat}  … n で次へ / N で前へ") };
                Ok(())
            }
            None => Err(format!("見つかりません: {pat}")),
        }
    }

    pub fn search_regex(&self) -> Option<Regex> {
        self.last_search.as_ref().and_then(|(p, _)| smartcase_regex(p))
    }
}

fn transform(s: &str, op: Op) -> String {
    match op {
        Op::Lower => s.to_lowercase(),
        Op::Upper => s.to_uppercase(),
        _ => s
            .chars()
            .map(|c| if c.is_uppercase() { c.to_lowercase().next().unwrap_or(c) } else { c.to_uppercase().next().unwrap_or(c) })
            .collect(),
    }
}

fn find_char(b: &dyn Buf, cur: usize, c: char, forward: bool, till: bool, n: usize, repeat: bool) -> Option<usize> {
    let l = b.line_of(cur);
    let (ls, le) = (b.line_start(l), b.line_end(l));
    let mut p = cur;
    for i in 0..n {
        loop {
            if forward {
                // t を繰り返すときは直前の位置で止まり続けないよう 1 文字飛ばす
                let skip = till && (repeat || i > 0) && b.next_pos(p) < le && b.char_at(b.next_pos(p)) == c;
                if skip {
                    p = b.next_pos(p);
                }
                p = b.next_pos(p);
                if p >= le {
                    return None;
                }
            } else {
                let skip = till && (repeat || i > 0) && p > ls && b.char_at(b.prev_pos(p)) == c;
                if skip {
                    p = b.prev_pos(p);
                }
                if p <= ls {
                    return None;
                }
                p = b.prev_pos(p);
            }
            if b.char_at(p) == c {
                break;
            }
        }
    }
    Some(if till { if forward { b.prev_pos(p) } else { b.next_pos(p) } } else { p })
}

fn match_pair(b: &dyn Buf, cur: usize) -> Option<usize> {
    let l = b.line_of(cur);
    let le = b.line_end(l);
    let mut p = cur;
    let pairs = [('(', ')'), ('[', ']'), ('{', '}'), ('<', '>')];
    while p < le {
        let c = b.char_at(p);
        if let Some(&(o, cl)) = pairs.iter().find(|(o, cl)| *o == c || *cl == c) {
            let forward = c == o;
            let mut depth = 0i32;
            let mut q = p;
            loop {
                let ch = b.char_at(q);
                if ch == o {
                    depth += if forward { 1 } else { -1 };
                } else if ch == cl {
                    depth += if forward { -1 } else { 1 };
                }
                if depth == 0 {
                    return Some(q);
                }
                if forward {
                    q = b.next_pos(q);
                    if q >= b.len() {
                        return None;
                    }
                } else {
                    if q == 0 {
                        return None;
                    }
                    q = b.prev_pos(q);
                }
            }
        }
        p = b.next_pos(p);
    }
    None
}

fn obj_word(b: &dyn Buf, cur: usize, big: bool, around: bool) -> Option<(usize, usize)> {
    let l = b.line_of(cur);
    let (ls, le) = (b.line_start(l), b.line_end(l));
    if ls == le {
        return None;
    }
    let cur = cur.min(b.prev_pos(le));
    let c = cls(b, cur, big);
    let mut s = cur;
    while s > ls && cls(b, b.prev_pos(s), big) == c {
        s = b.prev_pos(s);
    }
    let mut e = b.next_pos(cur);
    while e < le && cls(b, e, big) == c {
        e = b.next_pos(e);
    }
    if around {
        let e0 = e;
        while e < le && cls(b, e, big) == 0 {
            e = b.next_pos(e);
        }
        if e == e0 {
            while s > ls && cls(b, b.prev_pos(s), big) == 0 {
                s = b.prev_pos(s);
            }
        }
    }
    Some((s, e))
}

fn obj_quote(b: &dyn Buf, cur: usize, q: char, around: bool) -> Option<(usize, usize)> {
    let l = b.line_of(cur);
    let (ls, le) = (b.line_start(l), b.line_end(l));
    let mut quotes = Vec::new();
    let mut p = ls;
    let mut prev = '\0';
    while p < le {
        let c = b.char_at(p);
        if c == q && prev != '\\' {
            quotes.push(p);
        }
        prev = c;
        p = b.next_pos(p);
    }
    let pair = quotes
        .chunks_exact(2)
        .find(|w| w[0] <= cur && cur <= w[1])
        .or_else(|| quotes.chunks_exact(2).find(|w| w[0] > cur))?;
    let (a, z) = (pair[0], pair[1]);
    if around {
        let mut e = b.next_pos(z);
        while e < le && matches!(b.byte(e), b' ' | b'\t') {
            e += 1;
        }
        Some((a, e))
    } else {
        Some((b.next_pos(a), z))
    }
}

fn obj_pair(b: &dyn Buf, cur: usize, open: char, close: char, around: bool) -> Option<(usize, usize)> {
    // 開き括弧を後方へ探す
    let mut depth = 0i32;
    let mut p = cur;
    let start = loop {
        let c = b.char_at(p);
        if c == close && p != cur {
            depth += 1;
        } else if c == open {
            if depth == 0 {
                break p;
            }
            depth -= 1;
        }
        if p == 0 {
            return None;
        }
        p = b.prev_pos(p);
    };
    let mut depth = 0i32;
    let mut q = b.next_pos(start);
    let end = loop {
        if q >= b.len() {
            return None;
        }
        let c = b.char_at(q);
        if c == open {
            depth += 1;
        } else if c == close {
            if depth == 0 {
                break q;
            }
            depth -= 1;
        }
        q = b.next_pos(q);
    };
    if around { Some((start, b.next_pos(end))) } else { Some((b.next_pos(start), end)) }
}

// ---- パーサ ----

fn parse_count(keys: &[Key], i: &mut usize) -> Option<usize> {
    let mut n: Option<usize> = None;
    while let Some(Key::Char(c)) = keys.get(*i) {
        if c.is_ascii_digit() && (n.is_some() || *c != '0') {
            n = Some(n.unwrap_or(0).saturating_mul(10).saturating_add(c.to_digit(10).unwrap() as usize).min(999_999));
            *i += 1;
        } else {
            break;
        }
    }
    n
}

enum MP {
    Incomplete,
    Invalid,
    Done(Motion),
}

fn parse_motion(keys: &[Key], i: usize) -> MP {
    let Some(&k) = keys.get(i) else { return MP::Incomplete };
    let m = match k {
        Key::Char('h') | Key::Left | Key::Backspace => Motion::Left,
        Key::Char('l') | Key::Right | Key::Char(' ') => Motion::Right,
        Key::Char('j') | Key::Down | Key::Ctrl('n') => Motion::Down,
        Key::Char('k') | Key::Up | Key::Ctrl('p') => Motion::Up,
        Key::Char('w') => Motion::WordFwd(false),
        Key::Char('W') => Motion::WordFwd(true),
        Key::Char('e') => Motion::WordEnd(false),
        Key::Char('E') => Motion::WordEnd(true),
        Key::Char('b') => Motion::WordBack(false),
        Key::Char('B') => Motion::WordBack(true),
        Key::Char('0') | Key::Home => Motion::LineStart,
        Key::Char('^') => Motion::FirstNonBlank,
        Key::Char('$') | Key::End => Motion::LineEnd,
        Key::Char('G') => Motion::GotoLast,
        Key::Char('%') => Motion::MatchPair,
        Key::Char('}') => Motion::ParaFwd,
        Key::Char('{') => Motion::ParaBack,
        Key::Char('H') => Motion::ScreenTop,
        Key::Char('M') => Motion::ScreenMid,
        Key::Char('L') => Motion::ScreenBottom,
        Key::Char('n') => Motion::SearchNext(false),
        Key::Char('N') => Motion::SearchNext(true),
        Key::Char('*') => Motion::Star(true),
        Key::Char('#') => Motion::Star(false),
        Key::Char(';') => Motion::RepeatFind(false),
        Key::Char(',') => Motion::RepeatFind(true),
        Key::Char('+') | Key::Enter => Motion::NextLine,
        Key::Char('-') => Motion::PrevLine,
        Key::Char('g') => {
            return match keys.get(i + 1) {
                None => MP::Incomplete,
                Some(Key::Char('g')) => MP::Done(Motion::GotoFirst),
                Some(_) => MP::Invalid,
            };
        }
        Key::Char(c @ ('f' | 'F' | 't' | 'T')) => {
            return match keys.get(i + 1) {
                None => MP::Incomplete,
                Some(Key::Char(t)) => MP::Done(Motion::Find { c: *t, forward: c == 'f' || c == 't', till: c == 't' || c == 'T' }),
                Some(_) => MP::Invalid,
            };
        }
        _ => return MP::Invalid,
    };
    MP::Done(m)
}

fn parse_object(keys: &[Key], i: usize) -> Option<Option<Obj>> {
    let around = match keys.get(i)? {
        Key::Char('i') => false,
        Key::Char('a') => true,
        _ => return Some(None),
    };
    let Some(Key::Char(c)) = keys.get(i + 1) else {
        return if keys.get(i + 1).is_none() { None } else { Some(None) };
    };
    Some(Some(match c {
        'w' => Obj::Word { big: false, around },
        'W' => Obj::Word { big: true, around },
        '"' | '\'' | '`' => Obj::Quote { q: *c, around },
        '(' | ')' | 'b' => Obj::Pair { open: '(', close: ')', around },
        '{' | '}' | 'B' => Obj::Pair { open: '{', close: '}', around },
        '[' | ']' => Obj::Pair { open: '[', close: ']', around },
        '<' | '>' => Obj::Pair { open: '<', close: '>', around },
        _ => return Some(None),
    }))
}

fn op_of(keys: &[Key], i: usize) -> Option<Option<(Op, usize)>> {
    Some(Some(match keys.get(i)? {
        Key::Char('d') => (Op::Delete, 1),
        Key::Char('c') => (Op::Change, 1),
        Key::Char('y') => (Op::Yank, 1),
        Key::Char('>') => (Op::Indent, 1),
        Key::Char('<') => (Op::Outdent, 1),
        Key::Char('g') => match keys.get(i + 1) {
            None => return None,
            Some(Key::Char('u')) => (Op::Lower, 2),
            Some(Key::Char('U')) => (Op::Upper, 2),
            Some(Key::Char('~')) => (Op::Toggle, 2),
            _ => return Some(None),
        },
        _ => return Some(None),
    }))
}

fn done(count: Option<usize>, kind: CmdKind, keys: &[Key], body_from: usize) -> Parsed {
    Parsed::Done(Cmd { count, kind, body: keys[body_from..].to_vec() })
}

fn parse(keys: &[Key]) -> Parsed {
    let mut i = 0;
    let count = parse_count(keys, &mut i);
    let body = i;
    let Some(&k) = keys.get(i) else { return Parsed::Incomplete };
    // オペレータ
    match op_of(keys, i) {
        None => return Parsed::Incomplete,
        Some(Some((op, w))) => {
            let mut j = i + w;
            let c2 = parse_count(keys, &mut j);
            let count = match (count, c2) {
                (None, None) => None,
                (a, b) => Some(a.unwrap_or(1) * b.unwrap_or(1)),
            };
            let Some(&nk) = keys.get(j) else { return Parsed::Incomplete };
            // dd / cc / yy / >> / << / guu / gUU
            let doubled = match op {
                Op::Lower => nk == Key::Char('u') || (nk == Key::Char('g') && keys.get(j + 1) == Some(&Key::Char('u'))),
                Op::Upper => nk == Key::Char('U') || (nk == Key::Char('g') && keys.get(j + 1) == Some(&Key::Char('U'))),
                Op::Toggle => nk == Key::Char('~'),
                _ => Some(&nk) == keys.get(i),
            };
            if doubled {
                return done(count, CmdKind::Operate(op, Target::Line), keys, body);
            }
            if matches!(op, Op::Lower | Op::Upper) && nk == Key::Char('g') && keys.get(j + 1).is_none() {
                return Parsed::Incomplete;
            }
            match parse_object(keys, j) {
                None => return Parsed::Incomplete,
                Some(Some(o)) => return done(count, CmdKind::Operate(op, Target::Object(o)), keys, body),
                Some(None) => {}
            }
            return match parse_motion(keys, j) {
                MP::Incomplete => Parsed::Incomplete,
                MP::Invalid => Parsed::Invalid,
                MP::Done(m) => done(count, CmdKind::Operate(op, Target::Motion(m)), keys, body),
            };
        }
        Some(None) => {}
    }
    let act = |a| done(count, CmdKind::Act(a), keys, body);
    let op = |o, m| done(count, CmdKind::Operate(o, Target::Motion(m)), keys, body);
    match k {
        Key::Char('x') | Key::Del => return op(Op::Delete, Motion::Right),
        Key::Char('X') => return op(Op::Delete, Motion::Left),
        Key::Char('s') => return op(Op::Change, Motion::Right),
        Key::Char('S') => return done(count, CmdKind::Operate(Op::Change, Target::Line), keys, body),
        Key::Char('D') => return op(Op::Delete, Motion::LineEnd),
        Key::Char('C') => return op(Op::Change, Motion::LineEnd),
        Key::Char('Y') => return done(count, CmdKind::Operate(Op::Yank, Target::Line), keys, body),
        Key::Char('p') => return act(Act::PasteAfter),
        Key::Char('P') => return act(Act::PasteBefore),
        Key::Char('u') => return act(Act::Undo),
        Key::Ctrl('r') => return act(Act::Redo),
        Key::Char('.') => return act(Act::Repeat),
        Key::Char('J') => return act(Act::Join),
        Key::Char('i') => return act(Act::Insert(InsPos::Before)),
        Key::Char('I') => return act(Act::Insert(InsPos::LineStart)),
        Key::Char('a') => return act(Act::Insert(InsPos::After)),
        Key::Char('A') => return act(Act::Insert(InsPos::LineEnd)),
        Key::Char('o') => return act(Act::Insert(InsPos::Below)),
        Key::Char('O') => return act(Act::Insert(InsPos::Above)),
        Key::Char('v') => return act(Act::Visual),
        Key::Char('V') => return act(Act::VisualLine),
        Key::Char('~') => return act(Act::ToggleCase),
        Key::Char(c @ (':' | '/' | '?')) => return act(Act::Cmdline(c)),
        Key::Ctrl('d') => return act(Act::HalfDown),
        Key::Ctrl('u') => return act(Act::HalfUp),
        Key::Ctrl('f') => return act(Act::PageDown),
        Key::Ctrl('b') => return act(Act::PageUp),
        Key::Ctrl('e') => return act(Act::LineDown),
        Key::Ctrl('y') => return act(Act::LineUp),
        Key::Char('r') => {
            return match keys.get(i + 1) {
                None => Parsed::Incomplete,
                Some(Key::Char(c)) => act(Act::ReplaceChar(*c)),
                _ => Parsed::Invalid,
            };
        }
        Key::Char('Z') => {
            return match keys.get(i + 1) {
                None => Parsed::Incomplete,
                Some(Key::Char('Z')) => act(Act::SaveQuit),
                Some(Key::Char('Q')) => act(Act::QuitForce),
                _ => Parsed::Invalid,
            };
        }
        Key::Char('z') => {
            return match keys.get(i + 1) {
                None => Parsed::Incomplete,
                Some(Key::Char('z' | '.')) => act(Act::ScrollCenter),
                Some(Key::Char('t') | Key::Enter) => act(Act::ScrollTop),
                Some(Key::Char('b' | '-')) => act(Act::ScrollBottom),
                _ => Parsed::Invalid,
            };
        }
        _ => {}
    }
    match parse_motion(keys, i) {
        MP::Incomplete => Parsed::Incomplete,
        MP::Invalid => Parsed::Invalid,
        MP::Done(m) => done(count, CmdKind::Move(m), keys, body),
    }
}

fn parse_visual(keys: &[Key]) -> Parsed {
    let mut i = 0;
    let count = parse_count(keys, &mut i);
    let body = i;
    let Some(&k) = keys.get(i) else { return Parsed::Incomplete };
    let opk = |o| done(count, CmdKind::Operate(o, Target::Line), keys, body);
    let act = |a| done(count, CmdKind::Act(a), keys, body);
    match k {
        Key::Char('d' | 'x' | 'X' | 'D') | Key::Del => opk(Op::Delete),
        Key::Char('c' | 's' | 'C' | 'S' | 'R') => opk(Op::Change),
        Key::Char('y' | 'Y') => opk(Op::Yank),
        Key::Char('>') => opk(Op::Indent),
        Key::Char('<') => opk(Op::Outdent),
        Key::Char('~') => opk(Op::Toggle),
        Key::Char('u') => opk(Op::Lower),
        Key::Char('U') => opk(Op::Upper),
        Key::Char('o' | 'O') => act(Act::SwapEnds),
        Key::Char('v') => act(Act::Visual),
        Key::Char('V') => act(Act::VisualLine),
        Key::Char('J') => act(Act::Join),
        Key::Char('p' | 'P') => act(Act::PasteAfter),
        Key::Char(c @ (':' | '/' | '?')) => act(Act::Cmdline(c)),
        Key::Esc | Key::Ctrl('[') | Key::Ctrl('c') => Parsed::Invalid,
        _ => match parse_motion(keys, i) {
            MP::Incomplete => Parsed::Incomplete,
            MP::Invalid => Parsed::Invalid,
            MP::Done(m) => done(count, CmdKind::Move(m), keys, body),
        },
    }
}

#[cfg(test)]
mod tests;
