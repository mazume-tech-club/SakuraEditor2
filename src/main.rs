#![windows_subsystem = "windows"]

mod app;
mod config;
mod csv_fmt;
mod doc;
mod filter;
mod help;
mod json_fmt;
mod lang;
mod lexer_consts;
mod sci;
mod sci_buf;
mod sci_consts;
mod shell;
mod tail;
mod theme;
mod chrome;
mod session;
mod ui;
mod update;
mod util;
mod vi;

fn main() {
    let started = std::time::Instant::now();
    // 異常終了の原因を %LOCALAPPDATA%\Sakura2\crash.log に残す（コンソールが無いので標準エラーは見えない）
    std::panic::set_hook(Box::new(|info| {
        let line = format!("{} v{} {info}\n", util::timestamp(), env!("CARGO_PKG_VERSION"));
        let dir = util::local_data_dir();
        let _ = std::fs::create_dir_all(&dir);
        let _ = filter::output::append(&dir.join("crash.log"), line.as_bytes());
    }));
    unsafe {
        use windows_sys::Win32::UI::Controls::{ICC_BAR_CLASSES, ICC_STANDARD_CLASSES, ICC_TAB_CLASSES, INITCOMMONCONTROLSEX, InitCommonControlsEx};
        use windows_sys::Win32::UI::HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext};
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let icc = INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_TAB_CLASSES | ICC_BAR_CLASSES | ICC_STANDARD_CLASSES,
        };
        InitCommonControlsEx(&icc);
    }
    std::process::exit(app::run(started));
}
