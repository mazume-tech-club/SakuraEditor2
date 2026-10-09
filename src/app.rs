//! メインウィンドウ。タブ・メニュー・ステータスバー・フィルタ・tail・Vi をまとめる。

use std::cell::UnsafeCell;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    COLOR_BTNFACE, DeleteObject, GetSysColorBrush, HDC, HFONT, SetBkMode, SetTextColor, TRANSPARENT,
};
use windows_sys::Win32::UI::Controls::Dialogs::{
    GetOpenFileNameW, GetSaveFileNameW, OFN_EXPLORER, OFN_FILEMUSTEXIST, OFN_OVERWRITEPROMPT, OFN_PATHMUSTEXIST,
    OPENFILENAMEW,
};
use windows_sys::Win32::UI::Controls::{
    EM_SETSEL, NMHDR, SB_SETPARTS, SB_SETTEXTW, SBARS_SIZEGRIP, STATUSCLASSNAMEW, TCIF_PARAM, TCIF_TEXT,
    TCITEMW, TCM_DELETEITEM, TCM_GETCURSEL, TCM_INSERTITEMW, TCM_SETCURSEL, TCM_SETITEMW, TCN_SELCHANGE,
    TCS_FOCUSNEVER, TCS_TOOLTIPS, WC_TABCONTROLW,
};
use windows_sys::Win32::UI::HiDpi::GetDpiForWindow;
use windows_sys::Win32::UI::Input::Ime::{IACE_DEFAULT, ImmAssociateContextEx};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetFocus, GetKeyState, SetFocus, VK_BACK, VK_CONTROL, VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE, VK_F1, VK_F3,
    VK_F12, VK_F2, VK_HOME, VK_LEFT, VK_NEXT, VK_OEM_4, VK_PRIOR, VK_RETURN, VK_RIGHT, VK_TAB, VK_UP,
};
use windows_sys::Win32::UI::Shell::{DragAcceptFiles, DragFinish, DragQueryFileW, HDROP, ShellExecuteW};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use crate::config::Config;
use crate::doc::{self, Encoding, Eol};
use crate::filter::engine::{self, Compiled, FilterSpec, PatternSpec};
use crate::filter::highlight::{self, Highlighter};
use crate::filter::{self, output, Panel};
use crate::json_fmt;
use crate::lang::{self, Lang};
use crate::sci::{SCNotification, Sci};
use crate::sci_buf::SciBuf;
use crate::sci_consts::*;
use crate::tail::{self, Tail};
use crate::theme::{self, Palette};
use crate::ui;
use crate::update;
use crate::util::{self, w};
use crate::vi::{self, Key, Mode, Vi, ex::ExEffect};

// ---- メッセージ・ID ----
/// 画面に出す名前。exe・フォルダ・レジストリキーは Sakura2 のまま
pub const APP_NAME: &str = "さくらえでぃた弐";

const WM_APP_COUNT: u32 = WM_APP + 1;
const WM_APP_FILTER: u32 = WM_APP + 2;
const WM_APP_UPDATE: u32 = WM_APP + 3;
const WM_APP_STARTED: u32 = WM_APP + 4;

const TIMER_COUNT: usize = 1;
const TIMER_TAIL: usize = 2;
const TIMER_GREP: usize = 3;

const IDC_TAB: usize = 100;
const IDC_STATUS: usize = 101;
const IDC_CMD: usize = 102;
const IDC_CMDLABEL: usize = 103;
const IDC_SCI_BASE: usize = 1000;

mod cmd {
    pub const NEW: u16 = 40001;
    pub const OPEN: u16 = 40002;
    pub const SAVE: u16 = 40003;
    pub const SAVE_AS: u16 = 40004;
    pub const CLOSE_TAB: u16 = 40005;
    pub const RELOAD: u16 = 40006;
    pub const EXIT: u16 = 40007;
    pub const OPEN_FOLDER: u16 = 40008;
    pub const ENC_UTF8: u16 = 40020;
    pub const ENC_UTF8BOM: u16 = 40021;
    pub const ENC_SJIS: u16 = 40022;
    pub const ENC_UTF16LE: u16 = 40023;
    pub const REOPEN_UTF8: u16 = 40025;
    pub const REOPEN_SJIS: u16 = 40026;
    pub const EOL_CRLF: u16 = 40030;
    pub const EOL_LF: u16 = 40031;
    pub const UNDO: u16 = 40040;
    pub const REDO: u16 = 40041;
    pub const CUT: u16 = 40042;
    pub const COPY: u16 = 40043;
    pub const PASTE: u16 = 40044;
    pub const SELECT_ALL: u16 = 40045;
    pub const FIND: u16 = 40046;
    pub const FIND_NEXT: u16 = 40047;
    pub const FIND_PREV: u16 = 40048;
    pub const REPLACE: u16 = 40049;
    pub const GOTO: u16 = 40050;
    pub const JSON_FORMAT: u16 = 40051;
    pub const JSON_MINIFY: u16 = 40052;
    pub const FILTER_PANEL: u16 = 40060;
    pub const FILTER_RUN: u16 = 40061;
    pub const TAIL: u16 = 40062;
    pub const LOG_COLORS: u16 = 40063;
    pub const WRAP: u16 = 40064;
    pub const LINE_NUMBERS: u16 = 40065;
    pub const CLEAR_HL: u16 = 40066;
    pub const JUMP_SOURCE: u16 = 40067;
    pub const VI_MODE: u16 = 40070;
    pub const VI_HINTS: u16 = 40071;
    pub const AUTO_UPDATE: u16 = 40072;
    pub const OPEN_CONFIG: u16 = 40073;
    pub const CHEAT_SHEET: u16 = 40080;
    pub const GUIDE: u16 = 40084;
    pub const CHECK_UPDATE: u16 = 40081;
    pub const RESTART: u16 = 40082;
    pub const ABOUT: u16 = 40083;
    pub const GREP_CLEAR: u16 = 40068;
    pub const ADD_NEXT_MATCH: u16 = 40053;
    pub const SELECT_ALL_MATCHES: u16 = 40054;
    pub const CONTEXT_MENU: u16 = 40074;
    pub const NEXT_TAB: u16 = 40090;
    pub const PREV_TAB: u16 = 40091;
    pub const THEME_LIGHT: u16 = 40095;
    pub const THEME_DARK: u16 = 40096;
    pub const THEME_EDIT: u16 = 40097;
    pub const THEME_FOLDER: u16 = 40098;
    /// カスタムテーマは themes フォルダの一覧順に BASE から連番
    pub const THEME_CUSTOM_BASE: u16 = 40100;
    pub const THEME_CUSTOM_END: u16 = 40131;
}

// ---- 状態 ----

/// Ctrl+F の絞り込み表示（一致しない行を隠している状態）
pub struct Grep {
    pub pattern: String,
    pub compiled: Arc<Compiled>,
}

pub struct FilterLink {
    pub source: u32,
    pub source_path: Option<PathBuf>,
    /// 出力行 → 元の行番号
    pub map: Vec<u32>,
    /// 条件の連鎖（抽出結果をさらに抽出した場合）
    pub chain: Vec<Arc<Compiled>>,
    pub prefixed: bool,
    pub out: PathBuf,
}

pub struct Tab {
    pub id: u32,
    pub sci: Sci,
    pub path: Option<PathBuf>,
    pub enc: Encoding,
    pub eol: Eol,
    pub lang: Lang,
    pub tail: Option<Tail>,
    pub hl: Highlighter,
    pub link: Option<FilterLink>,
    pub grep: Option<Grep>,
    pub title: Option<String>,
    /// 読み込み/保存時のファイルサイズ（tail の開始位置）
    pub loaded_len: u64,
    margin_digits: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CmdMode {
    None,
    Ex,
    SearchFwd,
    SearchBack,
    Find,
    Replace,
    Goto,
}

struct CountResult {
    generation: u64,
    matched: usize,
    total: usize,
}

struct FilterDone {
    source: u32,
    source_path: Option<PathBuf>,
    chain: Vec<Arc<Compiled>>,
    prefixed: bool,
    summary: String,
    out: PathBuf,
    text: Vec<u8>,
    map: Vec<u32>,
    matched: usize,
    total: usize,
    ms: u128,
    error: Option<String>,
}

pub struct App {
    hwnd: HWND,
    hinst: *mut core::ffi::c_void,
    tabctl: HWND,
    status: HWND,
    cmd_edit: HWND,
    cmd_label: HWND,
    cmd_mode: CmdMode,
    font: HFONT,
    dpi: u32,
    tabs: Vec<Tab>,
    cur: usize,
    next_id: u32,
    panel: Panel,
    vi: Vi,
    cfg: Config,
    palette: Palette,
    theme_menu: HMENU,
    find_pat: Option<String>,
    count_gen: u64,
    started: Instant,
    bench: bool,
    bench_exit: bool,
    accel: HACCEL,
    updated_to: Option<String>,
    /// 初回描画が終わるまではフォーカス設定（IME 初期化で重い）を遅らせる
    ready: bool,
    shown_ms: f64,
    /// 非アクティブになる直前にフォーカスがあった子ウィンドウ
    last_focus: HWND,
}

struct Global(UnsafeCell<Option<App>>);
unsafe impl Sync for Global {}
static APP: Global = Global(UnsafeCell::new(None));

fn app_opt() -> Option<&'static mut App> {
    unsafe { (*APP.0.get()).as_mut() }
}

fn post<T>(hwnd: isize, msg: u32, wparam: usize, data: T) {
    let p = Box::into_raw(Box::new(data));
    unsafe {
        if PostMessageW(hwnd as HWND, msg, wparam, p as LPARAM) == 0 {
            drop(Box::from_raw(p));
        }
    }
}

unsafe fn take<T>(lparam: LPARAM) -> Box<T> {
    unsafe { Box::from_raw(lparam as *mut T) }
}

static mut TRACE: Vec<(&'static str, f64)> = Vec::new();
fn trace(started: Instant, label: &'static str) {
    unsafe { (*std::ptr::addr_of_mut!(TRACE)).push((label, started.elapsed().as_secs_f64() * 1000.0)) };
}

fn msgbox(hwnd: HWND, text: &str, flags: MESSAGEBOX_STYLE) -> MESSAGEBOX_RESULT {
    unsafe { MessageBoxW(hwnd, w(text).as_ptr(), w(APP_NAME).as_ptr(), flags) }
}

// ---- 起動 ----

pub fn run(started: Instant) -> i32 {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let bench = args.iter().any(|a| a == "--bench" || a == "--bench-exit");
    let bench_exit = args.iter().any(|a| a == "--bench-exit");
    let files: Vec<PathBuf> = args.iter().filter(|a| !a.starts_with("--")).map(PathBuf::from).collect();

    unsafe {
        let hinst = windows_sys::Win32::System::LibraryLoader::GetModuleHandleW(std::ptr::null());
        trace(started, "start");
        highlight::warm_up();
        crate::sci::register_classes(hinst);
        trace(started, "sci_register");
        let cfg = Config::load();
        let palette = Palette::load(&cfg.theme);
        trace(started, "config");

        let class = w("Sakura2Main");
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wndproc),
            hInstance: hinst,
            hIcon: LoadIconW(hinst, 1 as _),
            hIconSm: LoadImageW(hinst, 1 as _, IMAGE_ICON, GetSystemMetrics(SM_CXSMICON), GetSystemMetrics(SM_CYSMICON), LR_DEFAULTCOLOR) as HICON,
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            hbrBackground: GetSysColorBrush(COLOR_BTNFACE),
            lpszClassName: class.as_ptr(),
            ..std::mem::zeroed()
        };
        RegisterClassExW(&wc);
        trace(started, "register_class");
        let (menu, theme_menu) = build_menu();
        trace(started, "menu");
        let (x, y, cx, cy) = cfg.window.unwrap_or((CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT));
        let hwnd = CreateWindowExW(
            WS_EX_ACCEPTFILES,
            class.as_ptr(),
            w(APP_NAME).as_ptr(),
            WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN,
            x,
            y,
            cx,
            cy,
            std::ptr::null_mut(),
            menu,
            hinst,
            std::ptr::null(),
        );
        trace(started, "main_window");
        let dpi = GetDpiForWindow(hwnd).max(96);
        let font = ui::ui_font(dpi);
        trace(started, "font");
        let tabctl = ui::control(hwnd, hinst, 0, "SysTabControl32", "", WS_VISIBLE | WS_CLIPSIBLINGS | TCS_FOCUSNEVER | TCS_TOOLTIPS, IDC_TAB, font);
        let _ = WC_TABCONTROLW;
        // 末尾の「＋」は新規タブ用のボタン。実タブは常にこの手前に挿入する
        let plus = w(" ＋ ");
        let item = TCITEMW { mask: TCIF_TEXT, pszText: plus.as_ptr() as *mut u16, ..std::mem::zeroed() };
        SendMessageW(tabctl, TCM_INSERTITEMW, 0, &item as *const _ as LPARAM);
        let status = CreateWindowExW(0, STATUSCLASSNAMEW, std::ptr::null(), WS_CHILD | WS_VISIBLE | SBARS_SIZEGRIP, 0, 0, 0, 0, hwnd, IDC_STATUS as HMENU, hinst, std::ptr::null());
        ui::set_font(status, font);
        let cmd_label = ui::control(hwnd, hinst, 0, "STATIC", ":", ui::SS_LEFT, IDC_CMDLABEL, font);
        let cmd_edit = ui::control(hwnd, hinst, WS_EX_CLIENTEDGE, "EDIT", "", ES_AUTOHSCROLL as u32, IDC_CMD, font);
        trace(started, "tab_status_cmd");
        let panel = Panel::new(hwnd, hinst, font);
        trace(started, "controls");

        *APP.0.get() = Some(App {
            hwnd,
            hinst,
            tabctl,
            status,
            cmd_edit,
            cmd_label,
            cmd_mode: CmdMode::None,
            font,
            dpi,
            tabs: Vec::new(),
            cur: 0,
            next_id: 1,
            panel,
            vi: Vi::new(),
            cfg,
            palette,
            theme_menu,
            find_pat: None,
            count_gen: 0,
            started,
            bench,
            bench_exit,
            accel: build_accel(),
            updated_to: None,
            ready: false,
            shown_ms: 0.0,
            last_focus: std::ptr::null_mut(),
        });
        let app = app_opt().unwrap();
        if !app.cfg.filter.patterns.is_empty() {
            let spec = app.cfg.filter.clone();
            app.panel.set_spec(&spec);
        }
        app.new_tab();
        trace(started, "first_tab");
        for f in &files {
            app.open_path(f);
        }
        app.layout();
        DragAcceptFiles(hwnd, 1);
        ShowWindow(hwnd, if app.cfg.maximized { SW_SHOWMAXIMIZED } else { SW_SHOWNORMAL });
        windows_sys::Win32::Graphics::Gdi::UpdateWindow(hwnd);
        trace(started, "shown");
        app.shown_ms = started.elapsed().as_secs_f64() * 1000.0;
        PostMessageW(hwnd, WM_APP_STARTED, 0, 0);

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            let app = app_opt().unwrap();
            if app.pre_translate(&msg) {
                continue;
            }
            if TranslateAcceleratorW(hwnd, app.accel, &msg) != 0 {
                continue;
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        msg.wParam as i32
    }
}

/// 戻り値はメニューバーとテーマ用サブメニュー（項目は開くたびに作り直す）
fn build_menu() -> (HMENU, HMENU) {
    unsafe {
        let bar = CreateMenu();
        let add = |m: HMENU, id: u16, text: &str| {
            AppendMenuW(m, MF_STRING, id as usize, w(text).as_ptr());
        };
        let sep = |m: HMENU| {
            AppendMenuW(m, MF_SEPARATOR, 0, std::ptr::null());
        };
        let sub = |title: &str, m: HMENU| {
            AppendMenuW(bar, MF_POPUP, m as usize, w(title).as_ptr());
        };

        let file = CreatePopupMenu();
        add(file, cmd::NEW, "新しいタブ\tCtrl+N");
        add(file, cmd::OPEN, "開く...\tCtrl+O");
        add(file, cmd::RELOAD, "再読み込み\tCtrl+R");
        sep(file);
        add(file, cmd::SAVE, "上書き保存\tCtrl+S");
        add(file, cmd::SAVE_AS, "名前を付けて保存...\tCtrl+Shift+S");
        sep(file);
        let enc = CreatePopupMenu();
        add(enc, cmd::ENC_UTF8, "UTF-8 で保存");
        add(enc, cmd::ENC_UTF8BOM, "UTF-8 (BOM 付き) で保存");
        add(enc, cmd::ENC_SJIS, "SJIS で保存");
        add(enc, cmd::ENC_UTF16LE, "UTF-16LE で保存");
        sep(enc);
        add(enc, cmd::REOPEN_UTF8, "UTF-8 として開き直す");
        add(enc, cmd::REOPEN_SJIS, "SJIS として開き直す");
        sep(enc);
        add(enc, cmd::EOL_CRLF, "改行を CRLF に変換");
        add(enc, cmd::EOL_LF, "改行を LF に変換");
        AppendMenuW(file, MF_POPUP, enc as usize, w("文字コード・改行").as_ptr());
        add(file, cmd::OPEN_FOLDER, "フォルダを開く");
        sep(file);
        add(file, cmd::CLOSE_TAB, "タブを閉じる\tCtrl+W");
        add(file, cmd::EXIT, "終了\tAlt+F4");
        sub("ファイル(&F)", file);

        let edit = CreatePopupMenu();
        add(edit, cmd::UNDO, "元に戻す\tCtrl+Z");
        add(edit, cmd::REDO, "やり直し\tCtrl+Y");
        sep(edit);
        add(edit, cmd::CUT, "切り取り\tCtrl+X");
        add(edit, cmd::COPY, "コピー\tCtrl+C");
        add(edit, cmd::PASTE, "貼り付け\tCtrl+V");
        add(edit, cmd::SELECT_ALL, "すべて選択\tCtrl+A");
        sep(edit);
        add(edit, cmd::ADD_NEXT_MATCH, "次の一致を追加（複数カーソル）\tCtrl+D");
        add(edit, cmd::SELECT_ALL_MATCHES, "すべての一致を選択（複数カーソル）\tF2 / Alt+Enter");
        sep(edit);
        add(edit, cmd::JSON_FORMAT, "JSON を整形\tAlt+Shift+F");
        add(edit, cmd::JSON_MINIFY, "JSON を 1 行に圧縮\tAlt+Shift+M");
        sub("編集(&E)", edit);

        let search = CreatePopupMenu();
        add(search, cmd::FIND, "絞り込み検索（一致行だけ表示）...\tCtrl+F");
        add(search, cmd::GREP_CLEAR, "絞り込みを解除\tEsc");
        add(search, cmd::FIND_NEXT, "次を検索\tF3");
        add(search, cmd::FIND_PREV, "前を検索\tShift+F3");
        add(search, cmd::REPLACE, "置換...\tCtrl+H");
        add(search, cmd::GOTO, "指定行へ移動...\tCtrl+G");
        sep(search);
        add(search, cmd::FILTER_PANEL, "ログフィルタ パネル\tCtrl+Shift+L");
        add(search, cmd::FILTER_RUN, "フィルタを実行して Temp に出力\tCtrl+Shift+Enter");
        add(search, cmd::JUMP_SOURCE, "抽出結果から元の行へジャンプ\tF12 / ダブルクリック");
        add(search, cmd::CLEAR_HL, "ハイライトを消す");
        sub("検索(&S)", search);

        let view = CreatePopupMenu();
        add(view, cmd::TAIL, "ファイル追従 (tail -f)\tCtrl+Shift+T");
        add(view, cmd::LOG_COLORS, "ログレベルの色分け (ERROR/WARN)");
        add(view, cmd::WRAP, "折り返し\tAlt+Z");
        add(view, cmd::LINE_NUMBERS, "行番号");
        let theme_menu = CreatePopupMenu();
        AppendMenuW(view, MF_POPUP, theme_menu as usize, w("テーマ(&T)").as_ptr());
        sep(view);
        add(view, cmd::NEXT_TAB, "次のタブ\tCtrl+Tab");
        add(view, cmd::PREV_TAB, "前のタブ\tCtrl+Shift+Tab");
        sub("表示(&V)", view);

        let set = CreatePopupMenu();
        add(set, cmd::VI_MODE, "Vi モード\tCtrl+Alt+V");
        add(set, cmd::VI_HINTS, "Vi ヒントを表示（学習用）");
        sep(set);
        add(set, cmd::AUTO_UPDATE, "自動アップデート");
        add(set, cmd::CONTEXT_MENU, &format!("エクスプローラーの右クリックに「{}」", crate::shell::LABEL));
        add(set, cmd::OPEN_CONFIG, "設定ファイルを開く");
        sub("設定(&O)", set);

        let help = CreatePopupMenu();
        add(help, cmd::GUIDE, "操作ガイド\tF1");
        add(help, cmd::CHEAT_SHEET, "Vi チートシート\tShift+F1");
        sep(help);
        add(help, cmd::CHECK_UPDATE, "アップデートを確認");
        add(help, cmd::RESTART, "再起動（更新を適用）");
        add(help, cmd::ABOUT, "バージョン情報");
        sub("ヘルプ(&H)", help);
        (bar, theme_menu)
    }
}

fn build_accel() -> HACCEL {
    let v = |f: u8, key: u16, id: u16| ACCEL { fVirt: FVIRTKEY | f, key, cmd: id };
    let c = FCONTROL;
    let s = FSHIFT;
    let a = FALT;
    let k = |ch: char| ch as u16;
    let table = [
        v(c, k('N'), cmd::NEW),
        v(c, k('O'), cmd::OPEN),
        v(c, k('R'), cmd::RELOAD),
        v(c, k('S'), cmd::SAVE),
        v(c | s, k('S'), cmd::SAVE_AS),
        v(c, k('W'), cmd::CLOSE_TAB),
        v(c, k('F'), cmd::FIND),
        v(0, VK_F3, cmd::FIND_NEXT),
        v(s, VK_F3, cmd::FIND_PREV),
        v(c, k('H'), cmd::REPLACE),
        v(c, k('D'), cmd::ADD_NEXT_MATCH),
        v(0, VK_F2, cmd::SELECT_ALL_MATCHES),
        v(a, VK_RETURN, cmd::SELECT_ALL_MATCHES),
        v(c, k('G'), cmd::GOTO),
        v(a | s, k('F'), cmd::JSON_FORMAT),
        v(a | s, k('M'), cmd::JSON_MINIFY),
        v(c | s, k('L'), cmd::FILTER_PANEL),
        v(c | s, VK_RETURN, cmd::FILTER_RUN),
        v(c | s, k('T'), cmd::TAIL),
        v(a, k('Z'), cmd::WRAP),
        v(0, VK_F12, cmd::JUMP_SOURCE),
        v(c | a, k('V'), cmd::VI_MODE),
        v(0, VK_F1, cmd::GUIDE),
        v(s, VK_F1, cmd::CHEAT_SHEET),
        v(c, VK_TAB, cmd::NEXT_TAB),
        v(c | s, VK_TAB, cmd::PREV_TAB),
        v(c, VK_NEXT, cmd::NEXT_TAB),
        v(c, VK_PRIOR, cmd::PREV_TAB),
    ];
    unsafe { CreateAcceleratorTableW(table.as_ptr(), table.len() as i32) }
}

// ---- ウィンドウプロシージャ ----

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let Some(app) = app_opt() else {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    };
    match msg {
        WM_SIZE => {
            app.layout();
            0
        }
        WM_ACTIVATE => {
            if util::loword(wparam) as u32 == WA_INACTIVE {
                app.last_focus = unsafe { GetFocus() };
            }
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
        WM_SETFOCUS => {
            let last = app.last_focus;
            if !last.is_null() && unsafe { IsWindowVisible(last) } != 0 && unsafe { GetParent(last) } == hwnd {
                unsafe { SetFocus(last) };
            } else {
                app.focus_editor();
            }
            0
        }
        WM_COMMAND => {
            let id = util::loword(wparam) as usize;
            let code = util::hiword(wparam) as u32;
            if (filter::ID_BASE..=filter::ID_END).contains(&id) {
                app.on_panel(id, code);
            } else if id == IDC_CMD {
                if code == EN_CHANGE && app.cmd_mode == CmdMode::Find {
                    unsafe { SetTimer(hwnd, TIMER_GREP, 200, None) };
                }
            } else if lparam == 0 || id >= 40000 {
                app.on_command(id as u16);
            }
            0
        }
        WM_NOTIFY => {
            let hdr = unsafe { &*(lparam as *const NMHDR) };
            if hdr.hwndFrom == app.tabctl {
                if hdr.code == TCN_SELCHANGE {
                    let i = unsafe { SendMessageW(app.tabctl, TCM_GETCURSEL, 0, 0) };
                    if i as usize == app.tabs.len() {
                        app.new_tab();
                    } else if i >= 0 {
                        app.activate(i as usize);
                    }
                }
            } else if let Some(i) = app.tabs.iter().position(|t| t.sci.hwnd == hdr.hwndFrom) {
                app.on_sci_notify(i, unsafe { &*(lparam as *const SCNotification) });
            }
            0
        }
        WM_TIMER => {
            match wparam {
                TIMER_COUNT => {
                    unsafe { KillTimer(hwnd, TIMER_COUNT) };
                    app.update_count();
                }
                TIMER_TAIL => app.poll_tail(),
                TIMER_GREP => {
                    unsafe { KillTimer(hwnd, TIMER_GREP) };
                    if app.cmd_mode == CmdMode::Find {
                        let text = util::get_window_text(app.cmd_edit);
                        app.apply_grep(&text);
                    }
                }
                _ => {}
            }
            0
        }
        WM_CTLCOLORSTATIC => {
            let hdc = wparam as HDC;
            let ctl = lparam as HWND;
            if let Some(c) = app.panel.swatch_color(ctl) {
                unsafe {
                    SetTextColor(hdc, util::rgb(c) as u32);
                    SetBkMode(hdc, TRANSPARENT as i32);
                    return GetSysColorBrush(COLOR_BTNFACE) as LRESULT;
                }
            }
            if ctl == app.panel.count && app.panel.count_is_error {
                unsafe {
                    SetTextColor(hdc, util::rgb(0xD00000) as u32);
                    SetBkMode(hdc, TRANSPARENT as i32);
                    return GetSysColorBrush(COLOR_BTNFACE) as LRESULT;
                }
            }
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
        WM_INITMENUPOPUP => {
            if wparam as HMENU == app.theme_menu {
                app.update_theme_menu();
            } else {
                app.update_menu_checks(wparam as HMENU);
            }
            0
        }
        WM_DROPFILES => {
            let hdrop = wparam as HDROP;
            unsafe {
                let n = DragQueryFileW(hdrop, 0xFFFF_FFFF, std::ptr::null_mut(), 0);
                for i in 0..n {
                    let mut buf = [0u16; 1024];
                    DragQueryFileW(hdrop, i, buf.as_mut_ptr(), buf.len() as u32);
                    app.open_path(Path::new(&util::from_wide(&buf)));
                }
                DragFinish(hdrop);
            }
            0
        }
        WM_DPICHANGED => {
            app.dpi = util::hiword(wparam) as u32;
            let r = unsafe { &*(lparam as *const RECT) };
            unsafe {
                SetWindowPos(hwnd, std::ptr::null_mut(), r.left, r.top, r.right - r.left, r.bottom - r.top, SWP_NOZORDER | SWP_NOACTIVATE);
                let old = app.font;
                app.font = ui::ui_font(app.dpi);
                for h in [app.tabctl, app.status, app.cmd_edit, app.cmd_label] {
                    ui::set_font(h, app.font);
                }
                app.panel.set_font(app.font);
                DeleteObject(old);
            }
            app.layout();
            0
        }
        WM_APP_STARTED => {
            app.on_started();
            0
        }
        WM_APP_COUNT => {
            let r = unsafe { take::<CountResult>(lparam) };
            if r.generation == app.count_gen {
                app.panel.set_count(&format!("{} / {} 行が一致", fmt_num(r.matched), fmt_num(r.total)), false);
            }
            0
        }
        WM_APP_FILTER => {
            let r = unsafe { take::<FilterDone>(lparam) };
            app.on_filter_done(*r);
            0
        }
        WM_APP_UPDATE => {
            let r = unsafe { take::<update::Outcome>(lparam) };
            app.on_update(*r, wparam != 0);
            0
        }
        WM_CLOSE => {
            if app.confirm_close_all() {
                app.save_window_state();
                unsafe { DestroyWindow(hwnd) };
            }
            0
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

fn fmt_num(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

fn key_down(vk: u16) -> bool {
    unsafe { GetKeyState(vk as i32) < 0 }
}

impl App {
    fn tab(&self) -> &Tab {
        &self.tabs[self.cur]
    }

    fn sci(&self) -> Sci {
        self.tabs[self.cur].sci
    }

    fn tab_index(&self, id: u32) -> Option<usize> {
        self.tabs.iter().position(|t| t.id == id)
    }

    fn set_status(&self, part: usize, text: &str) {
        unsafe {
            SendMessageW(self.status, SB_SETTEXTW, part, w(text).as_ptr() as LPARAM);
        }
    }

    fn msg(&self, text: &str) {
        self.set_status(0, text);
    }

    // ---- 起動後の遅延処理 ----

    fn on_started(&mut self) {
        self.ready = true;
        self.focus_editor();
        let shown = self.shown_ms;
        let ms = self.started.elapsed().as_secs_f64() * 1000.0;
        self.msg(&format!("準備完了（表示 {shown:.0} ms）  F1: 操作ガイド / Ctrl+Shift+L: ログフィルタ"));
        if self.bench {
            let dir = util::local_data_dir();
            let _ = std::fs::create_dir_all(&dir);
            let tr: String = unsafe { (*std::ptr::addr_of!(TRACE)).iter().map(|(l, t)| format!(" {l}={t:.1}")).collect() };
            let _ = output::append(&dir.join("bench.log"), format!("{} shown_ms={shown:.2} ready_ms={ms:.2}{tr}\n", util::timestamp()).as_bytes());
            if self.bench_exit {
                unsafe { PostMessageW(self.hwnd, WM_CLOSE, 0, 0) };
                return;
            }
        }
        let context_menu = self.cfg.context_menu;
        std::thread::spawn(move || {
            update::cleanup_old_exe();
            output::cleanup_old();
            if context_menu {
                if let Ok(exe) = std::env::current_exe() {
                    crate::shell::ensure(&exe);
                }
            }
        });
        if self.cfg.auto_update {
            self.spawn_update(false);
        }
    }

    fn spawn_update(&self, manual: bool) {
        let hwnd = self.hwnd as isize;
        let repo = self.cfg.update_repo.clone();
        let token = self.cfg.github_token.clone();
        std::thread::spawn(move || {
            let r = update::check(&repo, &token, manual);
            post(hwnd, WM_APP_UPDATE, manual as usize, r);
        });
    }

    fn on_update(&mut self, r: update::Outcome, manual: bool) {
        match r {
            update::Outcome::Updated(v) => {
                self.msg(&format!("✔ {v} に更新しました。再起動すると反映されます（ヘルプ > 再起動）"));
                self.updated_to = Some(v);
            }
            update::Outcome::UpToDate if manual => {
                self.msg(&format!("最新版です（v{}）", update::current_version()));
            }
            update::Outcome::Failed(e) if manual => {
                self.msg(&format!("アップデート確認に失敗: {e}"));
            }
            _ => {}
        }
    }

    // ---- レイアウト ----

    fn layout(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        let mut rc: RECT = unsafe { std::mem::zeroed() };
        unsafe {
            GetClientRect(self.hwnd, &mut rc);
            SendMessageW(self.status, WM_SIZE, 0, 0);
        }
        let (wdt, hgt) = (rc.right, rc.bottom);
        let mut src: RECT = unsafe { std::mem::zeroed() };
        unsafe { GetWindowRect(self.status, &mut src) };
        let status_h = src.bottom - src.top;
        let s = |v| ui::scale(v, self.dpi);
        let tab_h = s(26);
        let cmd_h = if self.cmd_mode != CmdMode::None { s(26) } else { 0 };
        let panel_h = self.panel.height(self.dpi);
        let edit_bottom = hgt - status_h - cmd_h - panel_h;
        ui::place(self.tabctl, 0, 0, wdt, tab_h + s(2));
        let sci = self.sci();
        ui::place(sci.hwnd, 0, tab_h, wdt, edit_bottom - tab_h);
        self.panel.layout(0, edit_bottom, wdt, self.dpi);
        if cmd_h > 0 {
            let y = edit_bottom + panel_h;
            ui::place(self.cmd_label, s(6), y + s(5), s(90), s(18));
            ui::place(self.cmd_edit, s(98), y + s(2), wdt - s(104), s(22));
        }
        let parts = [wdt - s(600), wdt - s(460), wdt - s(290), wdt - s(210), wdt - s(130), wdt - s(80), -1];
        unsafe {
            SendMessageW(self.status, SB_SETPARTS, parts.len(), parts.as_ptr() as LPARAM);
        }
        self.update_status();
        let t = &mut self.tabs[self.cur];
        t.hl.refresh(&t.sci, true);
    }

    fn update_status(&self) {
        if self.tabs.is_empty() {
            return;
        }
        let t = self.tab();
        let sci = t.sci;
        let pos = sci.caret();
        let line = sci.line_of(pos);
        let col = sci.column(pos);
        let (a, b) = sci.sel_range();
        let cursors = sci.call(SCI_GETSELECTIONS, 0, 0) as usize;
        let sel = if cursors > 1 {
            format!("  カーソル {}", fmt_num(cursors))
        } else if b > a {
            format!("  選択 {}", b - a)
        } else {
            String::new()
        };
        let vi = if self.cfg.vi_mode {
            let p = self.vi.pending_label();
            if p.is_empty() { self.vi.mode.label().to_string() } else { format!("{}  {p}", self.vi.mode.label()) }
        } else {
            "通常モード".into()
        };
        self.set_status(1, &vi);
        self.set_status(2, &format!("{} 行, {} 列 / {} 行{sel}", fmt_num(line + 1), col + 1, fmt_num(sci.line_count())));
        self.set_status(3, t.lang.label());
        self.set_status(4, t.enc.label());
        self.set_status(5, t.eol.label());
        self.set_status(6, if t.tail.is_some() { "追従中" } else { "" });
    }

    fn update_margin(t: &mut Tab, show: bool) {
        let digits = if show { t.sci.line_count().to_string().len().max(3) } else { 0 };
        if digits != t.margin_digits {
            t.margin_digits = digits;
            let width = if show {
                let probe = format!("__{}\0", "9".repeat(digits));
                t.sci.call(SCI_TEXTWIDTH, STYLE_LINENUMBER as usize, probe.as_ptr() as isize)
            } else {
                0
            };
            t.sci.call(SCI_SETMARGINWIDTHN, 0, width);
        }
    }

    fn update_menu_checks(&self, menu: HMENU) {
        let check = |id: u16, on: bool| unsafe {
            CheckMenuItem(menu, id as u32, if on { MF_CHECKED } else { MF_UNCHECKED });
        };
        if self.tabs.is_empty() {
            return;
        }
        let t = self.tab();
        check(cmd::TAIL, t.tail.is_some());
        check(cmd::LOG_COLORS, self.cfg.log_levels);
        check(cmd::WRAP, self.cfg.wrap);
        check(cmd::LINE_NUMBERS, self.cfg.line_numbers);
        check(cmd::FILTER_PANEL, self.panel.visible);
        check(cmd::VI_MODE, self.cfg.vi_mode);
        check(cmd::VI_HINTS, self.cfg.vi_hints);
        check(cmd::AUTO_UPDATE, self.cfg.auto_update);
        check(cmd::CONTEXT_MENU, self.cfg.context_menu);
        check(cmd::ENC_UTF8, t.enc == Encoding::Utf8);
        check(cmd::ENC_UTF8BOM, t.enc == Encoding::Utf8Bom);
        check(cmd::ENC_SJIS, t.enc == Encoding::Sjis);
        check(cmd::ENC_UTF16LE, t.enc == Encoding::Utf16Le);
        check(cmd::EOL_CRLF, t.eol == Eol::Crlf);
        check(cmd::EOL_LF, t.eol == Eol::Lf);
    }

    // ---- タブ ----

    fn setup_sci(&self, sci: &Sci) {
        let c = &self.cfg;
        sci.call(SCI_SETCODEPAGE, SC_CP_UTF8 as usize, 0);
        if c.directwrite {
            sci.call(SCI_SETTECHNOLOGY, SC_TECHNOLOGY_DIRECTWRITE as usize, 0);
        }
        sci.call(SCI_SETMODEVENTMASK, 0, 0);
        sci.call_str(SCI_STYLESETFONT, STYLE_DEFAULT as usize, &c.font_name);
        sci.call(SCI_STYLESETSIZE, STYLE_DEFAULT as usize, c.font_size as isize);
        let p = &self.palette;
        sci.call(SCI_STYLESETFORE, STYLE_DEFAULT as usize, util::rgb(p.fg));
        sci.call(SCI_STYLESETBACK, STYLE_DEFAULT as usize, util::rgb(p.bg));
        sci.call(SCI_STYLECLEARALL, 0, 0);
        sci.call(SCI_SETTABWIDTH, c.tab_width as usize, 0);
        sci.call(SCI_SETUSETABS, 1, 0);
        sci.call(SCI_SETWRAPMODE, if c.wrap { SC_WRAP_CHAR } else { SC_WRAP_NONE } as usize, 0);
        sci.call(SCI_SETMARGINWIDTHN, 1, 0);
        sci.call(SCI_SETCARETLINEVISIBLE, 1, 0);
        sci.call(SCI_SETCARETLINEVISIBLEALWAYS, 1, 0);
        sci.call(SCI_SETCARETLINEBACK, util::rgb(p.caret_line) as usize, 0);
        sci.call(SCI_SETSELBACK, 1, util::rgb(p.selection));
        sci.call(SCI_SETADDITIONALSELBACK, util::rgb(p.selection) as usize, 0);
        sci.call(SCI_SETCARETFORE, util::rgb(p.caret) as usize, 0);
        sci.call(SCI_SETADDITIONALCARETFORE, util::rgb(p.caret) as usize, 0);
        sci.call(SCI_SETSCROLLWIDTHTRACKING, 1, 0);
        sci.call(SCI_SETSCROLLWIDTH, 1, 0);
        sci.call(SCI_SETMULTIPLESELECTION, 1, 0);
        sci.call(SCI_SETADDITIONALSELECTIONTYPING, 1, 0);
        sci.call(SCI_SETLAYOUTCACHE, SC_CACHE_PAGE as usize, 0);
        sci.call(SCI_SETENDATLASTLINE, 0, 0);
        sci.call(SCI_SETXCARETPOLICY, 0x05usize, 50);
        highlight::setup(sci, p);
        self.apply_caret_style(sci);
    }

    fn apply_caret_style(&self, sci: &Sci) {
        let block = self.cfg.vi_mode && self.vi.mode != Mode::Insert;
        sci.call(SCI_SETCARETSTYLE, if block { CARETSTYLE_BLOCK } else { CARETSTYLE_LINE } as usize, 0);
        sci.call(SCI_SETCARETWIDTH, 2, 0);
        unsafe {
            // ノーマルモードでは IME を切る（hjkl がそのまま効くように）
            if block {
                ImmAssociateContextEx(sci.hwnd, std::ptr::null_mut(), 0);
            } else {
                ImmAssociateContextEx(sci.hwnd, std::ptr::null_mut(), IACE_DEFAULT);
            }
        }
    }

    fn new_tab(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        let sci = Sci::create(self.hwnd, self.hinst, IDC_SCI_BASE + id as usize);
        self.setup_sci(&sci);
        sci.call(SCI_SETEOLMODE, SC_EOL_CRLF as usize, 0);
        lang::apply(&sci, Lang::Text, &self.palette);
        let mut tab = Tab {
            id,
            sci,
            path: None,
            enc: Encoding::Utf8,
            eol: Eol::Crlf,
            lang: Lang::Text,
            tail: None,
            hl: Highlighter::default(),
            link: None,
            grep: None,
            title: None,
            loaded_len: 0,
            margin_digits: usize::MAX,
        };
        Self::update_margin(&mut tab, self.cfg.line_numbers);
        tab.hl.set_log_levels(&sci, self.cfg.log_levels);
        let idx = self.tabs.len();
        self.tabs.push(tab);
        unsafe {
            let text = w("無題");
            let item = TCITEMW { mask: TCIF_TEXT | TCIF_PARAM, pszText: text.as_ptr() as *mut u16, lParam: id as LPARAM, ..std::mem::zeroed() };
            SendMessageW(self.tabctl, TCM_INSERTITEMW, idx, &item as *const _ as LPARAM);
        }
        self.activate(idx);
        idx
    }

    fn activate(&mut self, idx: usize) {
        if idx >= self.tabs.len() {
            return;
        }
        if idx != self.cur && self.cur < self.tabs.len() {
            ui::show(self.tabs[self.cur].sci.hwnd, false);
        }
        if self.cfg.vi_mode && idx != self.cur {
            let sci = self.tabs[self.cur.min(self.tabs.len() - 1)].sci;
            self.vi.reset(&mut SciBuf(&sci));
        }
        self.cur = idx;
        unsafe { SendMessageW(self.tabctl, TCM_SETCURSEL, idx, 0) };
        let sci = self.sci();
        self.apply_caret_style(&sci);
        ui::show(sci.hwnd, true);
        self.layout();
        self.focus_editor();
        self.update_title();
        if self.panel.visible {
            self.schedule_count();
        }
    }

    fn focus_editor(&self) {
        if self.ready && !self.tabs.is_empty() {
            unsafe { SetFocus(self.sci().hwnd) };
        }
    }

    fn tab_label(t: &Tab) -> String {
        let name = t
            .title
            .clone()
            .or_else(|| t.path.as_ref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "無題".into());
        let dirty = if t.sci.is_modified() { "*" } else { "" };
        let tail = if t.tail.is_some() { " ⟳" } else { "" };
        format!("{dirty}{name}{tail}")
    }

    fn update_tab_label(&self, i: usize) {
        let text = w(&Self::tab_label(&self.tabs[i]));
        unsafe {
            let item = TCITEMW { mask: TCIF_TEXT, pszText: text.as_ptr() as *mut u16, ..std::mem::zeroed() };
            SendMessageW(self.tabctl, TCM_SETITEMW, i, &item as *const _ as LPARAM);
        }
    }

    fn update_title(&self) {
        if self.tabs.is_empty() {
            return;
        }
        let t = self.tab();
        let name = t.path.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "無題".into());
        let dirty = if t.sci.is_modified() { "*" } else { "" };
        util::set_window_text(self.hwnd, &format!("{dirty}{name} - {APP_NAME}"));
        self.update_tab_label(self.cur);
    }

    fn close_tab(&mut self, idx: usize, force: bool) -> bool {
        if !force && !self.confirm_save(idx) {
            return false;
        }
        let t = self.tabs.remove(idx);
        unsafe {
            DestroyWindow(t.sci.hwnd);
            SendMessageW(self.tabctl, TCM_DELETEITEM, idx, 0);
        }
        if self.tabs.is_empty() {
            self.cur = 0;
            unsafe { PostMessageW(self.hwnd, WM_CLOSE, 0, 0) };
            return true;
        }
        self.cur = usize::MAX;
        self.activate(idx.min(self.tabs.len() - 1));
        self.ensure_tail_timer();
        true
    }

    /// 保存確認。続行してよければ true
    fn confirm_save(&mut self, idx: usize) -> bool {
        let t = &self.tabs[idx];
        if !t.sci.is_modified() || t.link.is_some() {
            return true;
        }
        let name = Self::tab_label(t);
        match msgbox(self.hwnd, &format!("{name} は変更されています。保存しますか？"), MB_YESNOCANCEL | MB_ICONQUESTION) {
            IDYES => self.save(idx, None),
            IDNO => true,
            _ => false,
        }
    }

    fn confirm_close_all(&mut self) -> bool {
        for i in 0..self.tabs.len() {
            if !self.confirm_save(i) {
                return false;
            }
        }
        true
    }

    fn save_window_state(&mut self) {
        unsafe {
            let mut wp: WINDOWPLACEMENT = std::mem::zeroed();
            wp.length = std::mem::size_of::<WINDOWPLACEMENT>() as u32;
            if GetWindowPlacement(self.hwnd, &mut wp) != 0 {
                let r = wp.rcNormalPosition;
                self.cfg.window = Some((r.left, r.top, r.right - r.left, r.bottom - r.top));
                self.cfg.maximized = wp.showCmd == SW_SHOWMAXIMIZED as u32;
            }
        }
        self.cfg.filter = self.panel.spec();
        self.cfg.save();
    }

    // ---- ファイル ----

    fn open_dialog(&self) -> Option<PathBuf> {
        let mut buf = vec![0u16; 4096];
        let filter: Vec<u16> = "すべてのファイル (*.*)\0*.*\0ログ (*.log;*.txt)\0*.log;*.txt\0JSON (*.json)\0*.json\0\0".encode_utf16().collect();
        let dir = self.tabs.get(self.cur).and_then(|t| t.path.as_ref()).and_then(|p| p.parent()).map(util::wpath);
        let mut ofn: OPENFILENAMEW = unsafe { std::mem::zeroed() };
        ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
        ofn.hwndOwner = self.hwnd;
        ofn.lpstrFilter = filter.as_ptr();
        ofn.lpstrFile = buf.as_mut_ptr();
        ofn.nMaxFile = buf.len() as u32;
        ofn.lpstrInitialDir = dir.as_ref().map(|d| d.as_ptr()).unwrap_or(std::ptr::null());
        ofn.Flags = OFN_EXPLORER | OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST;
        (unsafe { GetOpenFileNameW(&mut ofn) } != 0).then(|| PathBuf::from(util::from_wide(&buf)))
    }

    fn save_dialog(&self, idx: usize) -> Option<PathBuf> {
        let mut buf = vec![0u16; 4096];
        if let Some(name) = self.tabs[idx].path.as_ref().and_then(|p| p.file_name()) {
            for (i, c) in name.to_string_lossy().encode_utf16().take(4000).enumerate() {
                buf[i] = c;
            }
        }
        let filter: Vec<u16> = "すべてのファイル (*.*)\0*.*\0テキスト (*.txt)\0*.txt\0ログ (*.log)\0*.log\0JSON (*.json)\0*.json\0\0".encode_utf16().collect();
        let mut ofn: OPENFILENAMEW = unsafe { std::mem::zeroed() };
        ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
        ofn.hwndOwner = self.hwnd;
        ofn.lpstrFilter = filter.as_ptr();
        ofn.lpstrFile = buf.as_mut_ptr();
        ofn.nMaxFile = buf.len() as u32;
        ofn.Flags = OFN_EXPLORER | OFN_OVERWRITEPROMPT | OFN_PATHMUSTEXIST;
        (unsafe { GetSaveFileNameW(&mut ofn) } != 0).then(|| PathBuf::from(util::from_wide(&buf)))
    }

    fn open_path(&mut self, p: &Path) {
        let p = std::fs::canonicalize(p).map(|c| strip_unc(&c)).unwrap_or_else(|_| p.to_path_buf());
        if let Some(i) = self.tabs.iter().position(|t| t.path.as_deref() == Some(p.as_path())) {
            self.activate(i);
            return;
        }
        let raw = match std::fs::read(&p) {
            Ok(r) => r,
            Err(e) => {
                msgbox(self.hwnd, &format!("開けませんでした: {}\n{e}", p.display()), MB_ICONWARNING);
                return;
            }
        };
        let t = &self.tabs[self.cur];
        let reuse = t.path.is_none() && t.sci.len() == 0 && !t.sci.is_modified() && t.link.is_none();
        let idx = if reuse { self.cur } else { self.new_tab() };
        self.load_into(idx, &p, raw, None);
        self.activate(idx);
    }

    fn load_into(&mut self, idx: usize, p: &Path, raw: Vec<u8>, force_enc: Option<Encoding>) {
        let raw_len = raw.len() as u64;
        let (text, enc) = match force_enc {
            Some(e) => (doc::decode_chunk(&raw, e), e),
            None => doc::decode(raw),
        };
        let eol = doc::detect_eol(&text);
        let mut lang = Lang::from_path(p);
        if text.len() > 64 << 20 && !lang.is_log_like() {
            lang = Lang::Text;
        }
        let cfg_lines = self.cfg.line_numbers;
        let log_levels = self.cfg.log_levels;
        let t = &mut self.tabs[idx];
        t.sci.call(SCI_SETEOLMODE, match eol { Eol::Crlf => SC_EOL_CRLF, Eol::Lf => SC_EOL_LF, Eol::Cr => SC_EOL_CR } as usize, 0);
        lang::apply(&t.sci, lang, &self.palette);
        t.sci.load_bytes(&text);
        t.path = Some(p.to_path_buf());
        t.enc = enc;
        t.eol = eol;
        t.lang = lang;
        t.loaded_len = raw_len;
        if let Some(tail) = &mut t.tail {
            tail.offset = raw_len;
            tail.enc = enc;
        }
        Self::update_margin(t, cfg_lines);
        t.hl.set_log_levels(&t.sci, log_levels && lang.is_log_like());
        self.update_tab_label(idx);
        if idx == self.cur {
            self.update_title();
            self.update_status();
        }
    }

    fn reload(&mut self, force_enc: Option<Encoding>) {
        let idx = self.cur;
        let Some(p) = self.tabs[idx].path.clone() else { return };
        if self.tabs[idx].sci.is_modified()
            && msgbox(self.hwnd, "変更を破棄して読み込み直しますか？", MB_OKCANCEL | MB_ICONQUESTION) != IDOK
        {
            return;
        }
        let line = self.tabs[idx].sci.line_of(self.tabs[idx].sci.caret());
        match std::fs::read(&p) {
            Ok(raw) => {
                self.load_into(idx, &p, raw, force_enc);
                self.tabs[idx].sci.goto_line_centered(line);
            }
            Err(e) => self.msg(&format!("読み込み失敗: {e}")),
        }
    }

    fn save(&mut self, idx: usize, path: Option<PathBuf>) -> bool {
        let path = match path.or_else(|| self.tabs[idx].path.clone()) {
            Some(p) => p,
            None => match self.save_dialog(idx) {
                Some(p) => p,
                None => return false,
            },
        };
        let t = &self.tabs[idx];
        let data = doc::encode(t.sci.bytes(), t.enc);
        if let Err(e) = std::fs::write(&path, &data) {
            msgbox(self.hwnd, &format!("保存できませんでした: {}\n{e}", path.display()), MB_ICONERROR);
            return false;
        }
        let t = &mut self.tabs[idx];
        t.sci.call(SCI_SETSAVEPOINT, 0, 0);
        let lang_changed = t.path.as_ref() != Some(&path);
        t.path = Some(path.clone());
        t.loaded_len = data.len() as u64;
        if let Some(tail) = &mut t.tail {
            tail.offset = t.loaded_len;
        }
        if lang_changed && t.link.is_none() {
            t.lang = Lang::from_path(&path);
            lang::apply(&t.sci, t.lang, &self.palette);
        }
        self.update_tab_label(idx);
        self.update_title();
        self.msg(&format!("保存しました: {}", path.display()));
        if path == crate::config::path() {
            self.reload_config();
        } else if path == theme::path(&self.cfg.theme) {
            let name = self.cfg.theme.clone();
            self.apply_theme(&name);
        }
        true
    }

    fn reload_config(&mut self) {
        let old_vi = self.cfg.vi_mode;
        self.cfg = Config::load();
        self.palette = Palette::load(&self.cfg.theme);
        self.restyle_all();
        for t in &mut self.tabs {
            t.margin_digits = usize::MAX;
            Self::update_margin(t, self.cfg.line_numbers);
        }
        if old_vi != self.cfg.vi_mode {
            self.set_vi(self.cfg.vi_mode);
        }
        self.msg("設定を再読み込みしました");
    }

    /// 全タブの Scintilla 設定と配色をやり直す（フォント・テーマ変更後）
    fn restyle_all(&mut self) {
        for i in 0..self.tabs.len() {
            let sci = self.tabs[i].sci;
            self.setup_sci(&sci);
            lang::apply(&sci, self.tabs[i].lang, &self.palette);
        }
    }

    fn apply_theme(&mut self, name: &str) {
        if self.cfg.theme != name {
            self.cfg.theme = name.to_string();
            self.cfg.save();
        }
        self.palette = Palette::load(name);
        self.restyle_all();
        self.msg(&format!("テーマ: {name}"));
    }

    /// テーマを編集用にファイルとして開く。無ければ現在の配色から作る
    fn edit_theme(&mut self) {
        let name = match self.cfg.theme.as_str() {
            "light" | "dark" => "custom",
            n => n,
        }
        .to_string();
        let path = theme::path(&name);
        if !path.exists() {
            if let Err(e) = theme::save_theme(&name, &self.palette) {
                msgbox(self.hwnd, &format!("テーマを保存できませんでした: {}\n{e}", path.display()), MB_ICONERROR);
                return;
            }
        }
        self.apply_theme(&name);
        self.open_path(&path);
        self.msg(&format!("テーマ {name} を編集中。保存すると即座に反映されます（色は RRGGBB の 16 進）"));
    }

    fn update_theme_menu(&self) {
        unsafe {
            while DeleteMenu(self.theme_menu, 0, MF_BYPOSITION) != 0 {}
            let add = |id: u16, text: &str, on: bool| {
                AppendMenuW(self.theme_menu, MF_STRING | if on { MF_CHECKED } else { 0 }, id as usize, w(text).as_ptr());
            };
            add(cmd::THEME_LIGHT, "ライト", self.cfg.theme == "light");
            add(cmd::THEME_DARK, "ダーク", self.cfg.theme == "dark");
            let customs = Palette::list_custom();
            if !customs.is_empty() {
                AppendMenuW(self.theme_menu, MF_SEPARATOR, 0, std::ptr::null());
            }
            for (i, name) in customs.iter().take((cmd::THEME_CUSTOM_END - cmd::THEME_CUSTOM_BASE + 1) as usize).enumerate() {
                add(cmd::THEME_CUSTOM_BASE + i as u16, name, self.cfg.theme == *name);
            }
            AppendMenuW(self.theme_menu, MF_SEPARATOR, 0, std::ptr::null());
            add(cmd::THEME_EDIT, "テーマを編集（ファイルを開く）", false);
            add(cmd::THEME_FOLDER, "テーマ フォルダを開く", false);
        }
    }

    fn open_text_tab(&mut self, title: &str, text: &str, lang: Lang) {
        let idx = self.new_tab();
        let t = &mut self.tabs[idx];
        t.title = Some(title.into());
        lang::apply(&t.sci, lang, &self.palette);
        t.lang = lang;
        t.sci.load_bytes(text.replace('\n', "\r\n").as_bytes());
        t.sci.call(SCI_SETREADONLY, 1, 0);
        self.update_tab_label(idx);
        self.update_title();
    }

    // ---- コマンド ----

    fn on_command(&mut self, id: u16) {
        if self.tabs.is_empty() {
            return;
        }
        let sci = self.sci();
        match id {
            cmd::NEW => {
                self.new_tab();
            }
            cmd::OPEN => {
                if let Some(p) = self.open_dialog() {
                    self.open_path(&p);
                }
            }
            cmd::RELOAD => self.reload(None),
            cmd::SAVE => {
                self.save(self.cur, None);
            }
            cmd::SAVE_AS => {
                if let Some(p) = self.save_dialog(self.cur) {
                    self.save(self.cur, Some(p));
                }
            }
            cmd::CLOSE_TAB => {
                self.close_tab(self.cur, false);
            }
            cmd::EXIT => unsafe {
                PostMessageW(self.hwnd, WM_CLOSE, 0, 0);
            },
            cmd::OPEN_FOLDER => {
                if let Some(p) = &self.tab().path {
                    let arg = format!("/select,\"{}\"", p.display());
                    unsafe {
                        ShellExecuteW(self.hwnd, w("open").as_ptr(), w("explorer.exe").as_ptr(), w(&arg).as_ptr(), std::ptr::null(), SW_SHOWNORMAL);
                    }
                }
            }
            cmd::ENC_UTF8 | cmd::ENC_UTF8BOM | cmd::ENC_SJIS | cmd::ENC_UTF16LE => {
                let e = match id {
                    cmd::ENC_UTF8 => Encoding::Utf8,
                    cmd::ENC_UTF8BOM => Encoding::Utf8Bom,
                    cmd::ENC_SJIS => Encoding::Sjis,
                    _ => Encoding::Utf16Le,
                };
                self.tabs[self.cur].enc = e;
                self.msg(&format!("保存時の文字コードを {} にしました（保存で反映）", e.label()));
                self.update_status();
            }
            cmd::REOPEN_UTF8 => self.reload(Some(Encoding::Utf8)),
            cmd::REOPEN_SJIS => self.reload(Some(Encoding::Sjis)),
            cmd::EOL_CRLF | cmd::EOL_LF => {
                let (mode, eol) = if id == cmd::EOL_CRLF { (SC_EOL_CRLF, Eol::Crlf) } else { (SC_EOL_LF, Eol::Lf) };
                sci.call(SCI_SETEOLMODE, mode as usize, 0);
                sci.call(SCI_CONVERTEOLS, mode as usize, 0);
                self.tabs[self.cur].eol = eol;
                self.update_status();
            }
            cmd::UNDO => {
                sci.call(SCI_UNDO, 0, 0);
            }
            cmd::REDO => {
                sci.call(SCI_REDO, 0, 0);
            }
            cmd::CUT => {
                sci.call(SCI_CUT, 0, 0);
            }
            cmd::COPY => {
                sci.call(SCI_COPY, 0, 0);
            }
            cmd::PASTE => {
                sci.call(SCI_PASTE, 0, 0);
            }
            cmd::SELECT_ALL => {
                sci.call(SCI_SELECTALL, 0, 0);
            }
            cmd::FIND => {
                // 絞り込み中ならその条件、選択中ならその文字列、どちらでもなければ空で開く
                let sel = sci.sel_text();
                let pre = match &self.tab().grep {
                    Some(g) => g.pattern.clone(),
                    None if !sel.is_empty() && !sel.contains('\n') => regex::escape(&sel),
                    None => String::new(),
                };
                self.open_cmdline(CmdMode::Find, &pre);
                if !pre.is_empty() {
                    self.apply_grep(&pre);
                }
            }
            cmd::GREP_CLEAR => self.clear_grep(),
            cmd::ADD_NEXT_MATCH => self.select_matches(false),
            cmd::SELECT_ALL_MATCHES => self.select_matches(true),
            cmd::CONTEXT_MENU => {
                self.cfg.context_menu = !self.cfg.context_menu;
                self.cfg.save();
                if self.cfg.context_menu {
                    let ok = std::env::current_exe().map(|e| crate::shell::register(&e)).unwrap_or(false);
                    if ok {
                        self.msg(&format!("右クリックメニューに「{}」を追加しました（Windows 11 は「その他のオプションを確認」の中）", crate::shell::LABEL));
                    } else {
                        self.msg("⚠ 右クリックメニューの登録に失敗しました");
                    }
                } else {
                    crate::shell::unregister();
                    self.msg("右クリックメニューから削除しました");
                }
            }
            cmd::FIND_NEXT | cmd::FIND_PREV => {
                let pat = self.tab().grep.as_ref().map(|g| g.pattern.clone()).or_else(|| self.find_pat.clone());
                if let Some(p) = pat {
                    self.find(&p, id == cmd::FIND_NEXT);
                } else {
                    self.open_cmdline(CmdMode::Find, "");
                }
            }
            cmd::REPLACE => self.open_cmdline(CmdMode::Replace, ""),
            cmd::GOTO => self.open_cmdline(CmdMode::Goto, ""),
            cmd::JSON_FORMAT => self.json_format(),
            cmd::JSON_MINIFY => self.json_minify(),
            cmd::FILTER_PANEL => self.toggle_panel(),
            cmd::FILTER_RUN => {
                let spec = self.panel.spec();
                self.run_filter(spec);
            }
            cmd::TAIL => self.toggle_tail(),
            cmd::LOG_COLORS => {
                self.cfg.log_levels = !self.cfg.log_levels;
                let on = self.cfg.log_levels;
                for t in &mut self.tabs {
                    let s = t.sci;
                    t.hl.set_log_levels(&s, on && t.lang.is_log_like());
                }
            }
            cmd::WRAP => {
                self.cfg.wrap = !self.cfg.wrap;
                for t in &self.tabs {
                    t.sci.call(SCI_SETWRAPMODE, if self.cfg.wrap { SC_WRAP_CHAR } else { SC_WRAP_NONE } as usize, 0);
                }
            }
            cmd::LINE_NUMBERS => {
                self.cfg.line_numbers = !self.cfg.line_numbers;
                let on = self.cfg.line_numbers;
                for t in &mut self.tabs {
                    Self::update_margin(t, on);
                }
            }
            cmd::CLEAR_HL => {
                let t = &mut self.tabs[self.cur];
                let s = t.sci;
                t.hl.set_search(&s, None);
                t.hl.set_patterns(&s, Vec::new());
            }
            cmd::JUMP_SOURCE => self.jump_to_source(sci.line_of(sci.caret())),
            cmd::VI_MODE => self.set_vi(!self.cfg.vi_mode),
            cmd::VI_HINTS => {
                self.cfg.vi_hints = !self.cfg.vi_hints;
            }
            cmd::AUTO_UPDATE => {
                self.cfg.auto_update = !self.cfg.auto_update;
                self.cfg.save();
                self.msg(if self.cfg.auto_update { "自動アップデート: 有効" } else { "自動アップデート: 無効" });
            }
            cmd::OPEN_CONFIG => {
                self.save_window_state();
                let p = crate::config::path();
                self.open_path(&p);
                self.msg("設定を編集して保存すると即座に反映されます");
            }
            cmd::THEME_LIGHT => self.apply_theme("light"),
            cmd::THEME_DARK => self.apply_theme("dark"),
            cmd::THEME_EDIT => self.edit_theme(),
            cmd::THEME_FOLDER => theme::open_themes_folder(),
            cmd::THEME_CUSTOM_BASE..=cmd::THEME_CUSTOM_END => {
                let i = (id - cmd::THEME_CUSTOM_BASE) as usize;
                if let Some(name) = Palette::list_custom().get(i) {
                    self.apply_theme(name);
                }
            }
            cmd::GUIDE => self.open_text_tab("操作ガイド", crate::help::GUIDE, Lang::Text),
            cmd::CHEAT_SHEET => self.open_text_tab("Vi チートシート", vi::hints::CHEAT_SHEET, Lang::Text),
            cmd::CHECK_UPDATE => {
                self.msg("アップデートを確認しています...");
                self.spawn_update(true);
            }
            cmd::RESTART => self.restart(),
            cmd::ABOUT => {
                let text = format!(
                    "{} v{}\n\n高速起動のログ向けテキストエディタ\nサクラエディタ (sakura-editor) 本家とは無関係の非公式ソフトです\nライセンス: MIT OR Apache-2.0\n\n更新元: https://github.com/{}\n設定: {}\n{}",
                    APP_NAME,
                    update::current_version(),
                    self.cfg.update_repo,
                    crate::config::path().display(),
                    self.updated_to.as_ref().map(|v| format!("\n{v} への更新が適用待ちです（再起動で反映）")).unwrap_or_default()
                );
                msgbox(self.hwnd, &text, MB_ICONINFORMATION);
            }
            cmd::NEXT_TAB | cmd::PREV_TAB => {
                let n = self.tabs.len();
                let i = if id == cmd::NEXT_TAB { (self.cur + 1) % n } else { (self.cur + n - 1) % n };
                self.activate(i);
            }
            _ => {}
        }
        self.update_status();
    }

    fn restart(&mut self) {
        if !self.confirm_close_all() {
            return;
        }
        let files: Vec<String> = self.tabs.iter().filter(|t| t.link.is_none()).filter_map(|t| t.path.as_ref()).map(|p| p.display().to_string()).collect();
        if let Ok(exe) = std::env::current_exe() {
            let _ = std::process::Command::new(exe).args(files).spawn();
        }
        self.save_window_state();
        unsafe { DestroyWindow(self.hwnd) };
    }

    // ---- Scintilla 通知 ----

    fn on_sci_notify(&mut self, i: usize, n: &SCNotification) {
        match n.code {
            SCN_UPDATEUI => {
                if i == self.cur {
                    let show = self.cfg.line_numbers;
                    let t = &mut self.tabs[i];
                    if n.updated & (SC_UPDATE_CONTENT | SC_UPDATE_V_SCROLL | SC_UPDATE_H_SCROLL) as i32 != 0 {
                        let s = t.sci;
                        t.hl.refresh(&s, false);
                        Self::update_margin(t, show);
                    }
                    self.update_status();
                }
            }
            SCN_SAVEPOINTREACHED | SCN_SAVEPOINTLEFT => {
                self.update_tab_label(i);
                if i == self.cur {
                    self.update_title();
                }
            }
            SCN_DOUBLECLICK => {
                if self.tabs[i].link.is_some() {
                    let line = self.tabs[i].sci.line_of(n.position.max(0) as usize);
                    self.jump_to_source(line);
                }
            }
            _ => {}
        }
    }

    fn jump_to_source(&mut self, line: usize) {
        let Some(link) = &self.tabs[self.cur].link else {
            self.msg("抽出結果のタブで使えます");
            return;
        };
        let Some(&src_line) = link.map.get(line) else { return };
        let (source, path) = (link.source, link.source_path.clone());
        let idx = match self.tab_index(source) {
            Some(i) => i,
            None => match path {
                Some(p) => {
                    self.open_path(&p);
                    self.cur
                }
                None => {
                    self.msg("元のタブが閉じられています");
                    return;
                }
            },
        };
        self.activate(idx);
        let sci = self.sci();
        sci.goto_line_centered(src_line as usize);
        let s = sci.line_start(src_line as usize);
        sci.set_sel(s, s);
        self.msg(&format!("元ファイルの {} 行目へジャンプしました", src_line + 1));
    }

    // ---- フィルタ ----

    fn toggle_panel(&mut self) {
        let focus = unsafe { GetFocus() };
        if self.panel.visible && self.panel.owns(focus) {
            self.close_panel();
            return;
        }
        if !self.panel.visible {
            let sel = self.sci().sel_text();
            if !sel.is_empty() && !sel.contains('\n') {
                self.panel.set_first_text(&regex::escape(&sel));
            }
            self.panel.show(true);
            self.layout();
        }
        self.panel.focus();
        self.schedule_count();
    }

    fn close_panel(&mut self) {
        self.panel.show(false);
        let t = &mut self.tabs[self.cur];
        if t.link.is_none() {
            let s = t.sci;
            t.hl.set_patterns(&s, Vec::new());
        }
        self.layout();
        self.focus_editor();
    }

    fn on_panel(&mut self, id: usize, code: u32) {
        match self.panel.on_command(id, code) {
            filter::Event::None => {}
            filter::Event::Changed => self.schedule_count(),
            filter::Event::Run => {
                let spec = self.panel.spec();
                self.run_filter(spec);
            }
            filter::Event::Close => self.close_panel(),
            filter::Event::Relayout => {
                self.layout();
                self.schedule_count();
            }
        }
    }

    fn schedule_count(&self) {
        unsafe { SetTimer(self.hwnd, TIMER_COUNT, 150, None) };
    }

    fn update_count(&mut self) {
        let spec = self.panel.spec();
        self.count_gen += 1;
        let sci = self.sci();
        let compiled = match engine::compile(&spec) {
            Ok(c) => Arc::new(c),
            Err(e) => {
                let empty = e == "条件が空です";
                self.panel.set_count(if empty { "" } else { &e }, !empty);
                let t = &mut self.tabs[self.cur];
                t.hl.set_patterns(&sci, Vec::new());
                return;
            }
        };
        let pats = compiled.highlights.clone();
        self.tabs[self.cur].hl.set_patterns(&sci, pats);
        self.panel.set_count("集計中…", false);
        let data = sci.bytes().to_vec();
        let generation = self.count_gen;
        let hwnd = self.hwnd as isize;
        std::thread::spawn(move || {
            let (matched, total) = compiled.count(&data);
            post(hwnd, WM_APP_COUNT, 0, CountResult { generation, matched, total });
        });
    }

    fn run_filter(&mut self, spec: FilterSpec) {
        let compiled = match engine::compile(&spec) {
            Ok(c) => Arc::new(c),
            Err(e) => {
                self.msg(&format!("フィルタ: {e}"));
                if self.panel.visible {
                    self.panel.set_count(&e, true);
                }
                return;
            }
        };
        let t = &self.tabs[self.cur];
        // 抽出結果をさらに抽出した場合は元ファイルに遡れるよう対応表を合成する
        let (source, source_path, mut chain, src_map, prefixed) = match &t.link {
            Some(l) => (l.source, l.source_path.clone(), l.chain.clone(), Some(l.map.clone()), l.prefixed),
            None => (t.id, t.path.clone(), Vec::new(), None, false),
        };
        chain.push(compiled.clone());
        let line_numbers = spec.line_numbers && !prefixed;
        let data = t.sci.bytes().to_vec();
        let out = output::output_path(source_path.as_deref());
        let summary = spec.summary();
        let hwnd = self.hwnd as isize;
        self.msg("フィルタ実行中…");
        std::thread::spawn(move || {
            let t0 = Instant::now();
            let mut hits = compiled.matching_lines(&data, 0);
            if let Some(m) = &src_map {
                for h in &mut hits {
                    h.0 = m.get(h.0 as usize).copied().unwrap_or(h.0);
                }
            }
            let total = memchr::memchr_iter(b'\n', &data).count() + 1;
            let (text, map) = engine::build_output(&data, &hits, line_numbers);
            let error = output::write(&out, &text).err().map(|e| e.to_string());
            post(hwnd, WM_APP_FILTER, 0, FilterDone {
                source,
                source_path,
                chain,
                prefixed: prefixed || line_numbers,
                summary,
                out,
                matched: map.len(),
                text,
                map,
                total,
                ms: t0.elapsed().as_millis(),
                error,
            });
        });
    }

    fn on_filter_done(&mut self, r: FilterDone) {
        let idx = self.new_tab();
        let highlights: Vec<_> = r.chain.iter().flat_map(|c| c.highlights.iter().cloned()).collect();
        let log_levels = self.cfg.log_levels;
        let t = &mut self.tabs[idx];
        t.sci.call(SCI_SETEOLMODE, SC_EOL_CRLF as usize, 0);
        lang::apply(&t.sci, Lang::Log, &self.palette);
        t.lang = Lang::Log;
        t.sci.load_bytes(&r.text);
        t.path = Some(r.out.clone());
        t.title = Some(format!("[抽出] {}", r.summary.chars().take(24).collect::<String>()));
        t.loaded_len = r.text.len() as u64;
        let s = t.sci;
        t.hl.set_log_levels(&s, log_levels);
        t.hl.set_patterns(&s, highlights);
        t.margin_digits = usize::MAX;
        t.link = Some(FilterLink {
            source: r.source,
            source_path: r.source_path,
            map: r.map,
            chain: r.chain,
            prefixed: r.prefixed,
            out: r.out.clone(),
        });
        Self::update_margin(t, self.cfg.line_numbers);
        self.update_tab_label(idx);
        self.update_title();
        let where_ = match &r.error {
            Some(e) => format!("（ファイル出力失敗: {e}）"),
            None => format!("→ {}", r.out.display()),
        };
        self.msg(&format!(
            "{} / {} 行を抽出 {where_}  [{} ms]  ダブルクリック/F12 で元の行へ",
            fmt_num(r.matched),
            fmt_num(r.total),
            r.ms
        ));
    }

    // ---- tail ----

    fn toggle_tail(&mut self) {
        let t = &mut self.tabs[self.cur];
        if t.tail.is_some() {
            t.tail = None;
            t.sci.call(SCI_SETUNDOCOLLECTION, 1, 0);
            self.msg("ファイル追従を停止しました");
        } else {
            let Some(_) = &t.path else {
                self.msg("ファイルを開いてから追従できます");
                return;
            };
            t.tail = Some(Tail { offset: t.loaded_len, enc: t.enc });
            t.sci.call(SCI_SETUNDOCOLLECTION, 0, 0);
            t.sci.call(SCI_EMPTYUNDOBUFFER, 0, 0);
            t.sci.call(SCI_DOCUMENTEND, 0, 0);
            self.msg("ファイル追従中（tail -f）… 末尾にいる間は自動スクロールします");
            self.poll_tail();
        }
        self.update_tab_label(self.cur);
        self.ensure_tail_timer();
        self.update_status();
    }

    fn ensure_tail_timer(&self) {
        unsafe {
            if self.tabs.iter().any(|t| t.tail.is_some()) {
                SetTimer(self.hwnd, TIMER_TAIL, 500, None);
            } else {
                KillTimer(self.hwnd, TIMER_TAIL);
            }
        }
    }

    fn poll_tail(&mut self) {
        for i in 0..self.tabs.len() {
            let (Some(path), Some(_)) = (self.tabs[i].path.clone(), self.tabs[i].tail.as_ref()) else { continue };
            let ev = tail::poll(&path, self.tabs[i].tail.as_mut().unwrap());
            match ev {
                tail::Event::None => {}
                tail::Event::Truncated => {
                    if let Ok(raw) = std::fs::read(&path) {
                        self.load_into(i, &path, raw, None);
                        self.tabs[i].sci.call(SCI_DOCUMENTEND, 0, 0);
                        self.msg("ファイルが切り詰められた（ローテーション）ので読み直しました");
                    }
                }
                tail::Event::Appended(bytes) => self.append_tail(i, &bytes),
            }
        }
    }

    fn append_tail(&mut self, i: usize, bytes: &[u8]) {
        let sci = self.tabs[i].sci;
        let id = self.tabs[i].id;
        let was_clean = !sci.is_modified();
        let lines_before = sci.line_count();
        let first = sci.first_visible_doc_line();
        let follow = first + sci.lines_on_screen() + 1 >= lines_before || sci.line_of(sci.caret()) + 1 >= lines_before;
        let base_line = lines_before - 1;
        sci.append(bytes);
        if was_clean {
            sci.call(SCI_SETSAVEPOINT, 0, 0);
        }
        if let Some(g) = &self.tabs[i].grep {
            let hits = g.compiled.matching_lines(bytes, base_line);
            hide_except(&sci, base_line, sci.line_count() - 1, hits.iter().map(|h| h.0 as usize));
        }
        if follow {
            sci.call(SCI_DOCUMENTEND, 0, 0);
            sci.call(SCI_SCROLLCARET, 0, 0);
        }
        // この元ファイルにつながる抽出結果タブにも追記する
        for j in 0..self.tabs.len() {
            let Some(link) = &self.tabs[j].link else { continue };
            if link.source != id || link.chain.is_empty() {
                continue;
            }
            let mut hits = link.chain[0].matching_lines(bytes, base_line);
            hits.retain(|h| link.chain[1..].iter().all(|c| c.line_matches(&bytes[h.1..h.2])));
            // 位置は bytes 基準
            if hits.is_empty() {
                continue;
            }
            let (text, map) = engine::build_output(bytes, &hits, link.prefixed);
            let out = link.out.clone();
            let rs = self.tabs[j].sci;
            let r_clean = !rs.is_modified();
            let r_follow = rs.line_of(rs.caret()) + 2 >= rs.line_count();
            rs.call(SCI_SETUNDOCOLLECTION, 0, 0);
            // 最終行が空でなければ（改行で終わっていなければ）そのまま後ろに付く
            rs.append(&text);
            rs.call(SCI_SETUNDOCOLLECTION, 1, 0);
            if r_clean {
                rs.call(SCI_SETSAVEPOINT, 0, 0);
            }
            if r_follow {
                rs.call(SCI_DOCUMENTEND, 0, 0);
            }
            let _ = output::append(&out, &text);
            if let Some(l) = &mut self.tabs[j].link {
                l.map.extend(map);
            }
        }
        let t = &mut self.tabs[i];
        let s = t.sci;
        t.hl.refresh(&s, true);
        if i == self.cur {
            self.update_status();
            if self.panel.visible {
                self.schedule_count();
            }
        }
    }

    // ---- 検索・置換 ----

    /// Ctrl+F の絞り込み表示。一致しない行を隠し、一致箇所をハイライトする（空なら解除）
    fn apply_grep(&mut self, pat: &str) {
        if pat.is_empty() {
            self.clear_grep();
            return;
        }
        let compiled = match engine::compile(&grep_spec(pat, false)) {
            Ok(c) => Arc::new(c),
            Err(e) => {
                self.msg(&format!("⚠ {e}"));
                return;
            }
        };
        let sci = self.sci();
        let t0 = Instant::now();
        let hits = compiled.matching_lines(sci.bytes(), 0);
        let total = sci.line_count();
        if hits.is_empty() {
            self.show_all_lines();
            let t = &mut self.tabs[self.cur];
            t.grep = None;
            t.hl.set_search(&sci, None);
            self.msg(&format!("一致する行がありません: {pat}"));
            return;
        }
        unsafe { SendMessageW(sci.hwnd, WM_SETREDRAW, 0, 0) };
        sci.call(SCI_SHOWLINES, 0, total as isize - 1);
        hide_except(&sci, 0, total - 1, hits.iter().map(|h| h.0 as usize));
        unsafe {
            SendMessageW(sci.hwnd, WM_SETREDRAW, 1, 0);
            windows_sys::Win32::Graphics::Gdi::InvalidateRect(sci.hwnd, std::ptr::null(), 1);
        }
        // 行の先頭ではなく一致そのものを選択する（1 行が長いテキストでも F3 で続けて移れる）
        let re = compiled.highlights.first().map(|(r, _)| r.clone());
        let found = re.as_ref().and_then(|re| self.goto_match(re, sci.sel_range().0, true));
        sci.call(SCI_SCROLLCARET, 0, 0);
        let t = &mut self.tabs[self.cur];
        t.hl.set_search(&sci, re);
        t.grep = Some(Grep { pattern: pat.to_string(), compiled });
        self.find_pat = Some(pat.to_string());
        let pos = found.map(|(n, i, _)| format!("  {} 件中 {} 件目", fmt_num(n), fmt_num(i))).unwrap_or_default();
        self.msg(&format!(
            "絞り込み: {} / {} 行{pos} [{} ms]  Esc か空欄 Enter で解除 / Ctrl+Enter で Temp に出力 / F3 で次へ",
            fmt_num(hits.len()),
            fmt_num(total),
            t0.elapsed().as_millis()
        ));
        self.update_status();
    }

    fn show_all_lines(&self) {
        let sci = self.sci();
        sci.call(SCI_SHOWLINES, 0, sci.line_count() as isize - 1);
    }

    fn clear_grep(&mut self) {
        let sci = self.sci();
        let had = self.tabs[self.cur].grep.take().is_some();
        if had {
            self.show_all_lines();
            sci.call(SCI_SCROLLCARET, 0, 0);
        }
        self.tabs[self.cur].hl.set_search(&sci, None);
        self.find_pat = None;
        if had {
            self.msg("絞り込みを解除しました");
        }
    }

    fn find(&mut self, pat: &str, forward: bool) {
        self.find_pat = Some(pat.to_string());
        let re = {
            let ci = !pat.chars().any(|c| c.is_uppercase());
            regex::bytes::RegexBuilder::new(pat).case_insensitive(ci).multi_line(true).build()
        };
        let re = match re {
            Ok(r) => r,
            Err(e) => {
                self.msg(&format!("正規表現エラー: {e}"));
                return;
            }
        };
        let sci = self.sci();
        let (a, b) = sci.sel_range();
        match self.goto_match(&re, if forward { b } else { a }, forward) {
            Some((n, i, wrapped)) => {
                sci.call(SCI_SCROLLCARET, 0, 0);
                let pos = format!("{} 件中 {} 件目", fmt_num(n), fmt_num(i));
                self.msg(&if wrapped {
                    format!("{pos}  端に達したので反対側から検索しました")
                } else {
                    format!("{pos}  F3: 次 / Shift+F3: 前")
                });
            }
            None => self.msg(&format!("見つかりません: {pat}")),
        }
        let t = &mut self.tabs[self.cur];
        t.hl.set_search(&sci, Some(re));
    }

    /// from 以降（forward=false なら from より前）の一致を選択する。
    /// 戻り値は (一致の総数, 何件目か, 端で折り返したか)
    fn goto_match(&mut self, re: &regex::bytes::Regex, from: usize, forward: bool) -> Option<(usize, usize, bool)> {
        let sci = self.sci();
        let buf = SciBuf(&sci);
        use crate::vi::buf::Buf;
        // Buf::search の前方検索は from の次の文字から探すので、1 文字戻して from 自身も対象にする
        let (s, e, wrapped) = if forward && from == 0 {
            re.find(sci.bytes()).map(|m| (m.start(), m.end(), false))?
        } else {
            buf.search(re, if forward { buf.prev_pos(from) } else { from }, forward)?
        };
        if self.cfg.vi_mode && self.vi.mode != Mode::Insert {
            sci.goto(s);
        } else {
            sci.set_sel(s, e);
        }
        let (mut n, mut i) = (0, 0);
        for m in re.find_iter(sci.bytes()) {
            n += 1;
            if m.start() <= s {
                i = n;
            }
        }
        Some((n, i, wrapped))
    }

    /// 複数カーソル: 選択中の文字列（無ければキャレット位置の単語）の一致を選択に加える。
    /// all=false なら次の 1 件、true ならすべて
    fn select_matches(&mut self, all: bool) {
        if self.cfg.vi_mode && self.vi.mode != Mode::Insert {
            self.msg("複数カーソルは挿入モードで使えます（i で挿入モード）");
            return;
        }
        let sci = self.sci();
        sci.call(SCI_SETSEARCHFLAGS, SCFIND_MATCHCASE as usize, 0);
        sci.call(SCI_TARGETWHOLEDOCUMENT, 0, 0);
        // 選択が空なら Scintilla がキャレット位置の単語を選択する（以後は単語単位で一致を探す）
        if sci.call(SCI_GETSELECTIONEMPTY, 0, 0) != 0 {
            sci.call(SCI_MULTIPLESELECTADDNEXT, 0, 0);
            if sci.call(SCI_GETSELECTIONEMPTY, 0, 0) != 0 {
                self.msg("記号などは先に選択してから実行してください（例: ; を選択して F2）");
                return;
            }
            if !all {
                self.after_select_matches();
                return;
            }
        }
        if all {
            let n = count_occurrences(sci.bytes(), sci.sel_text().as_bytes());
            if n > MAX_CURSORS {
                self.msg(&format!("一致が多すぎます（上限 {} 件）。Ctrl+H の置換を使ってください", fmt_num(MAX_CURSORS)));
                return;
            }
            sci.call(SCI_MULTIPLESELECTADDEACH, 0, 0);
        } else {
            sci.call(SCI_MULTIPLESELECTADDNEXT, 0, 0);
        }
        self.after_select_matches();
    }

    fn after_select_matches(&mut self) {
        let sci = self.sci();
        sci.call(SCI_SCROLLCARET, 0, 0);
        let n = sci.call(SCI_GETSELECTIONS, 0, 0) as usize;
        self.msg(&format!("{} 箇所を選択中  入力すると全箇所に反映 / Ctrl+D で次を追加 / Esc で解除", fmt_num(n)));
        self.update_status();
    }

    fn open_cmdline(&mut self, mode: CmdMode, prefill: &str) {
        self.cmd_mode = mode;
        let label = match mode {
            CmdMode::Ex => ":",
            CmdMode::SearchFwd => "/",
            CmdMode::SearchBack => "?",
            CmdMode::Find => "検索(正規表現):",
            CmdMode::Replace => "置換 検索/置換:",
            CmdMode::Goto => "行番号:",
            CmdMode::None => "",
        };
        util::set_window_text(self.cmd_label, label);
        util::set_window_text(self.cmd_edit, prefill);
        ui::show(self.cmd_label, true);
        ui::show(self.cmd_edit, true);
        self.layout();
        unsafe {
            SetFocus(self.cmd_edit);
            SendMessageW(self.cmd_edit, EM_SETSEL, if mode == CmdMode::Ex { prefill.len() } else { 0 }, -1);
        }
        if mode == CmdMode::Replace {
            self.msg("例: ERROR/エラー  （正規表現、\\1 や & で参照可。全体を置換します）");
        }
    }

    fn close_cmdline(&mut self) {
        self.cmd_mode = CmdMode::None;
        ui::show(self.cmd_label, false);
        ui::show(self.cmd_edit, false);
        self.layout();
        self.focus_editor();
    }

    fn exec_cmdline(&mut self) {
        let text = util::get_window_text(self.cmd_edit);
        let mode = self.cmd_mode;
        self.close_cmdline();
        let sci = self.sci();
        match mode {
            CmdMode::Ex => self.exec_ex(&text),
            CmdMode::SearchFwd | CmdMode::SearchBack => {
                let r = self.vi.search(&mut SciBuf(&sci), &text, mode == CmdMode::SearchFwd);
                match r {
                    Ok(()) => self.msg(&self.vi.hint.clone()),
                    Err(e) => self.msg(&e),
                }
                sci.call(SCI_SCROLLCARET, 0, 0);
                let re = self.vi.search_regex();
                self.tabs[self.cur].hl.set_search(&sci, re);
            }
            CmdMode::Find => self.apply_grep(&text),
            CmdMode::Replace => {
                if !text.is_empty() {
                    let cmd = format!("%s/{text}/g");
                    self.exec_ex(&cmd);
                }
            }
            CmdMode::Goto => {
                if let Ok(n) = text.trim().parse::<usize>() {
                    sci.goto_line_centered(n.max(1) - 1);
                }
            }
            CmdMode::None => {}
        }
        self.update_status();
    }

    fn exec_ex(&mut self, text: &str) {
        let sci = self.sci();
        let r = vi::ex::run(&mut self.vi, &mut SciBuf(&sci), text);
        match r {
            ExEffect::None => {}
            ExEffect::Msg(m) => self.msg(&m),
            ExEffect::Error(e) => self.msg(&format!("⚠ {e}")),
            ExEffect::Save(p) => {
                self.save(self.cur, p.map(PathBuf::from));
            }
            ExEffect::SaveQuit => {
                if !sci.is_modified() || self.save(self.cur, None) {
                    self.close_tab(self.cur, true);
                }
            }
            ExEffect::Quit { force } => {
                self.close_tab(self.cur, force);
            }
            ExEffect::Edit(p) => self.open_path(Path::new(&p)),
            ExEffect::Filter { pattern, exclude } => {
                let spec = FilterSpec {
                    patterns: vec![PatternSpec {
                        case_sensitive: pattern.chars().any(|c| c.is_uppercase()),
                        text: pattern,
                        exclude,
                        regex: true,
                        color: 0,
                    }],
                    line_numbers: self.panel.spec().line_numbers,
                };
                self.run_filter(spec);
            }
            ExEffect::Set(opt) => self.set_option(&opt),
            ExEffect::NoHighlight => {
                self.clear_grep();
                self.tabs[self.cur].hl.set_search(&sci, None);
            }
            ExEffect::Tail => self.toggle_tail(),
            ExEffect::Json => self.json_format(),
            ExEffect::ViOff => self.set_vi(false),
        }
        if !self.tabs.is_empty() {
            let t = &mut self.tabs[self.cur];
            let s = t.sci;
            t.hl.refresh(&s, true);
        }
    }

    fn set_option(&mut self, opt: &str) {
        match opt {
            "nu" | "number" => {
                self.cfg.line_numbers = true;
            }
            "nonu" | "nonumber" => {
                self.cfg.line_numbers = false;
            }
            "wrap" | "nowrap" => {
                self.cfg.wrap = opt == "wrap";
                for t in &self.tabs {
                    t.sci.call(SCI_SETWRAPMODE, if self.cfg.wrap { SC_WRAP_CHAR } else { SC_WRAP_NONE } as usize, 0);
                }
                return;
            }
            "list" | "nolist" => {
                let v = if opt == "list" { SCWS_VISIBLEALWAYS } else { SCWS_INVISIBLE };
                self.sci().call(SCI_SETVIEWWS, v as usize, 0);
                self.sci().call(SCI_SETVIEWEOL, (opt == "list") as usize, 0);
                return;
            }
            _ => {
                self.msg(&format!("⚠ 未対応のオプション: {opt}（nu / nonu / wrap / nowrap / list / nolist）"));
                return;
            }
        }
        let on = self.cfg.line_numbers;
        for t in &mut self.tabs {
            Self::update_margin(t, on);
        }
    }

    // ---- JSON ----

    fn json_indent(&self) -> Vec<u8> {
        if self.cfg.json_indent == 0 { b"\t".to_vec() } else { vec![b' '; self.cfg.json_indent as usize] }
    }

    fn json_format(&mut self) {
        let sci = self.sci();
        let indent = self.json_indent();
        let eol = sci.eol_str();
        let (a, b) = sci.sel_range();
        let (start, end, base_line) = if b > a { (a, b, sci.line_of(a)) } else { (0, sci.len(), 0) };
        let text = sci.text_range(start, end);
        match json_fmt::format(&text, &indent, eol) {
            Ok(out) => {
                sci.begin_undo();
                sci.replace_range(start, end, &out);
                sci.end_undo();
                if b > a {
                    sci.set_sel(start, start + out.len());
                } else {
                    sci.goto(0);
                    let t = &mut self.tabs[self.cur];
                    if t.lang == Lang::Text {
                        t.lang = Lang::Json;
                        lang::apply(&sci, Lang::Json, &self.palette);
                    }
                }
                self.msg("JSON を整形しました（Alt+Shift+M で 1 行に圧縮）");
            }
            Err(e) => {
                // ログ行に埋め込まれた JSON を探す
                if b == a {
                    let line = sci.line_of(sci.caret());
                    let (ls, le) = (sci.line_start(line), sci.line_end(line));
                    let lt = sci.text_range(ls, le);
                    if let Some((s, e2, out)) = json_fmt::format_embedded(&lt, &indent, eol) {
                        sci.begin_undo();
                        sci.replace_range(ls + s, ls + e2, &out);
                        sci.end_undo();
                        sci.goto(ls + s);
                        self.msg("カーソル行に含まれる JSON を整形しました");
                        return;
                    }
                }
                let line = base_line + e.line.saturating_sub(1);
                let pos = sci.line_start(line) + e.column.saturating_sub(1);
                sci.goto(pos.min(sci.line_end(line)));
                sci.call(SCI_SCROLLCARET, 0, 0);
                self.msg(&format!("⚠ JSON エラー（{} 行 {} 列）: {}", line + 1, e.column, e.message));
            }
        }
    }

    fn json_minify(&mut self) {
        let sci = self.sci();
        let (a, b) = sci.sel_range();
        let (start, end) = if b > a { (a, b) } else { (0, sci.len()) };
        match json_fmt::minify(&sci.text_range(start, end)) {
            Ok(out) => {
                sci.begin_undo();
                sci.replace_range(start, end, &out);
                sci.end_undo();
                self.msg("JSON を 1 行に圧縮しました");
            }
            Err(e) => self.msg(&format!("⚠ JSON エラー（{} 行 {} 列）: {}", e.line, e.column, e.message)),
        }
    }

    // ---- Vi ----

    fn set_vi(&mut self, on: bool) {
        self.cfg.vi_mode = on;
        let sci = self.sci();
        self.vi.reset(&mut SciBuf(&sci));
        for t in &self.tabs {
            self.apply_caret_style(&t.sci);
        }
        self.msg(if on {
            "Vi モード ON（i で挿入、Esc で戻る、:q で閉じる、Shift+F1 でチートシート）"
        } else {
            "Vi モード OFF（通常のエディタ操作）"
        });
        self.cfg.save();
        self.update_status();
    }

    fn vi_key(&mut self, k: Key) {
        let sci = self.sci();
        let before = self.vi.mode;
        let out = self.vi.key(&mut SciBuf(&sci), k);
        match out.effect {
            vi::Effect::None => {}
            vi::Effect::Cmdline(c, pre) => {
                let mode = match c {
                    '/' => CmdMode::SearchFwd,
                    '?' => CmdMode::SearchBack,
                    _ => CmdMode::Ex,
                };
                self.open_cmdline(mode, &pre);
            }
            vi::Effect::Ex(s) => self.exec_ex(&s),
            vi::Effect::Message(m) => self.msg(&m),
        }
        if self.tabs.is_empty() {
            return;
        }
        let sci = self.sci();
        if before != self.vi.mode {
            self.apply_caret_style(&sci);
        }
        sci.call(SCI_SCROLLCARET, 0, 0);
        if self.cfg.vi_hints && !self.vi.hint.is_empty() {
            self.msg(&self.vi.hint.clone());
        }
        if matches!(k, Key::Char('n' | 'N' | '*' | '#')) {
            let re = self.vi.search_regex();
            let t = &mut self.tabs[self.cur];
            t.hl.set_search(&sci, re);
        }
        self.update_status();
    }

    /// メッセージループでの先取り処理。true なら処理済み
    fn pre_translate(&mut self, msg: &MSG) -> bool {
        if self.tabs.is_empty() {
            return false;
        }
        let vk = msg.wParam as u16;
        if msg.hwnd == self.cmd_edit && msg.message == WM_KEYDOWN {
            match vk {
                VK_RETURN if self.cmd_mode == CmdMode::Find && key_down(VK_CONTROL) => {
                    // Ctrl+Enter: 絞り込み結果を Temp に出力
                    let text = util::get_window_text(self.cmd_edit);
                    self.close_cmdline();
                    if !text.is_empty() {
                        self.run_filter(grep_spec(&text, self.panel.spec().line_numbers));
                    }
                    return true;
                }
                VK_ESCAPE if self.cmd_mode == CmdMode::Find => {
                    self.close_cmdline();
                    self.clear_grep();
                    return true;
                }
                VK_RETURN => {
                    self.exec_cmdline();
                    return true;
                }
                VK_ESCAPE => {
                    self.close_cmdline();
                    return true;
                }
                _ => return false,
            }
        }
        if self.panel.is_edit(msg.hwnd) && msg.message == WM_KEYDOWN {
            match vk {
                VK_RETURN => {
                    let spec = self.panel.spec();
                    self.run_filter(spec);
                    return true;
                }
                VK_ESCAPE => {
                    self.close_panel();
                    return true;
                }
                _ => return false,
            }
        }
        // 抽出結果タブでは Enter でもジャンプ（Vi モードでない場合）
        if !self.cfg.vi_mode {
            if msg.message == WM_KEYDOWN && vk == VK_ESCAPE && msg.hwnd == self.sci().hwnd && self.tab().grep.is_some() {
                self.clear_grep();
                return true;
            }
            if msg.message == WM_KEYDOWN && vk == VK_ESCAPE && msg.hwnd == self.sci().hwnd && self.panel.visible {
                self.close_panel();
                return true;
            }
            return false;
        }
        if msg.hwnd != self.sci().hwnd {
            return false;
        }
        let ctrl = key_down(VK_CONTROL);
        match msg.message {
            WM_KEYDOWN => {
                if self.vi.mode == Mode::Insert {
                    if vk == VK_ESCAPE || (ctrl && vk == VK_OEM_4) {
                        self.vi_key(Key::Esc);
                        return true;
                    }
                    return false;
                }
                let special = match vk {
                    VK_ESCAPE => Some(Key::Esc),
                    VK_LEFT => Some(Key::Left),
                    VK_RIGHT => Some(Key::Right),
                    VK_UP => Some(Key::Up),
                    VK_DOWN => Some(Key::Down),
                    VK_HOME => Some(Key::Home),
                    VK_END => Some(Key::End),
                    VK_DELETE => Some(Key::Del),
                    VK_RETURN => Some(Key::Enter),
                    VK_BACK => Some(Key::Backspace),
                    _ => None,
                };
                if let Some(k) = special {
                    if !ctrl {
                        self.vi_key(k);
                        return true;
                    }
                    return false;
                }
                if ctrl {
                    if vk == VK_OEM_4 {
                        self.vi_key(Key::Esc);
                        return true;
                    }
                    let c = (vk as u8 as char).to_ascii_lowercase();
                    if "rdufbeynp".contains(c) && !key_down(windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_SHIFT) {
                        self.vi_key(Key::Ctrl(c));
                        return true;
                    }
                    return false;
                }
                // 文字キーは WM_CHAR にしてから処理する（Scintilla には渡さない）
                unsafe { TranslateMessage(msg) };
                true
            }
            WM_CHAR => {
                if self.vi.mode == Mode::Insert {
                    return false;
                }
                if let Some(c) = char::from_u32(msg.wParam as u32) {
                    if !c.is_control() {
                        self.vi_key(Key::Char(c));
                    }
                }
                true
            }
            _ => false,
        }
    }
}

/// \\?\C:\... を C:\... に戻す
fn strip_unc(p: &Path) -> PathBuf {
    let s = p.to_string_lossy();
    match s.strip_prefix(r"\\?\") {
        Some(r) if !r.starts_with("UNC\\") => PathBuf::from(r),
        Some(r) => PathBuf::from(format!(r"\\{}", &r[4..])),
        None => p.to_path_buf(),
    }
}

/// Ctrl+F 用の 1 条件フィルタ（大文字を含めば大小を区別 = smartcase）
fn grep_spec(pat: &str, line_numbers: bool) -> FilterSpec {
    FilterSpec {
        patterns: vec![PatternSpec {
            text: pat.to_string(),
            regex: true,
            case_sensitive: pat.chars().any(|c| c.is_uppercase()),
            ..Default::default()
        }],
        line_numbers,
    }
}

/// すべての一致を選択するときのカーソル数の上限（多すぎると入力のたびに重くなる）
const MAX_CURSORS: usize = 10_000;

/// hay に含まれる needle の個数（重なりは数えない）。上限を超えたら数えるのをやめる
fn count_occurrences(hay: &[u8], needle: &[u8]) -> usize {
    if needle.is_empty() {
        return 0;
    }
    let mut n = 0;
    let mut i = 0;
    while let Some(p) = hay[i..].windows(needle.len()).position(|w| w == needle) {
        n += 1;
        i += p + needle.len();
        if n > MAX_CURSORS {
            break;
        }
    }
    n
}

/// [from, to] の行のうち keep に含まれない行を隠す（keep は昇順）
fn hide_except(sci: &Sci, from: usize, to: usize, keep: impl Iterator<Item = usize>) {
    let mut next = from;
    for l in keep {
        if l < next || l > to {
            continue;
        }
        if l > next {
            sci.call(SCI_HIDELINES, next, l as isize - 1);
        }
        next = l + 1;
    }
    if next <= to {
        sci.call(SCI_HIDELINES, next, to as isize);
    }
}
