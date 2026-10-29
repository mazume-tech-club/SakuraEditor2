//! ダークテーマ時のウィンドウ枠の描画（タイトルバー・メニューバー・タブ・ステータスバー・子コントロール）

use windows_sys::Win32::Foundation::{HWND, LPARAM, RECT};
use windows_sys::Win32::Graphics::Dwm::{
    DWMWA_CAPTION_COLOR, DWMWA_TEXT_COLOR, DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute,
};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, CreateSolidBrush, DT_CENTER, DT_END_ELLIPSIS, DT_HIDEPREFIX, DT_LEFT, DT_NOPREFIX, DT_SINGLELINE,
    DT_VCENTER, DeleteObject, DrawTextW, EndPaint, FillRect, GetWindowDC, HBRUSH, HDC, HGDIOBJ, PAINTSTRUCT,
    ReleaseDC, SelectObject, SetBkMode, SetTextColor, TRANSPARENT,
};
use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows_sys::Win32::UI::Controls::{
    DRAWITEMSTRUCT, ODS_DISABLED, ODS_HOTLIGHT, ODS_INACTIVE, ODS_NOACCEL, ODS_SELECTED, SB_GETPARTS, SB_GETRECT,
    SB_GETTEXTW, SetWindowTheme, TCIF_TEXT, TCITEMW, TCM_GETCURSEL, TCM_GETITEMCOUNT, TCM_GETITEMRECT, TCM_GETITEMW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GetClientRect, GetMenuBarInfo, GetMenuItemInfoW, GetWindowLongW, GetWindowRect, HMENU, MENUBARINFO,
    MENUITEMINFOW, MIIM_STRING, OBJID_MENU, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
    SendMessageW, SetWindowLongW, SetWindowPos, WM_GETFONT, WS_EX_CLIENTEDGE,
};

use crate::theme::Palette;
use crate::util::{rgb, w};

/// メニューバーの描画要求（非公開メッセージ。Windows 10 1809 以降）
pub const WM_UAHDRAWMENU: u32 = 0x0091;
pub const WM_UAHDRAWMENUITEM: u32 = 0x0092;

#[repr(C)]
struct UahMenu {
    hmenu: HMENU,
    hdc: HDC,
    flags: u32,
}

#[repr(C)]
struct UahMenuItem {
    position: i32,
    metrics: [[u32; 2]; 4],
    popup_cx: [u32; 4],
    update_max_widths: u32,
}

#[repr(C)]
struct UahDrawMenuItem {
    dis: DRAWITEMSTRUCT,
    um: UahMenu,
    umi: UahMenuItem,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Button,
    CheckBox,
    Edit,
    Scintilla,
}

pub struct Brushes {
    pub ui: HBRUSH,
    pub edit: HBRUSH,
}

impl Brushes {
    pub fn none() -> Self {
        Brushes { ui: std::ptr::null_mut(), edit: std::ptr::null_mut() }
    }

    pub fn create(p: &Palette) -> Self {
        unsafe { Brushes { ui: CreateSolidBrush(rgb(p.ui_bg) as u32), edit: CreateSolidBrush(rgb(p.bg) as u32) } }
    }

    pub fn release(&mut self) {
        unsafe {
            for b in [&mut self.ui, &mut self.edit] {
                if !b.is_null() {
                    DeleteObject(*b as HGDIOBJ);
                    *b = std::ptr::null_mut();
                }
            }
        }
    }
}

/// タイトルバーをテーマに合わせる。色指定は Windows 11 のみ有効で、それ以外では無視される
pub fn set_title_bar(hwnd: HWND, p: &Palette, dark: bool) {
    unsafe {
        let on: i32 = dark as i32;
        DwmSetWindowAttribute(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE as u32, &on as *const _ as _, 4);
        const DEFAULT: u32 = 0xFFFF_FFFF;
        let (cap, txt) = if dark { (rgb(p.ui_bg) as u32, rgb(p.ui_fg) as u32) } else { (DEFAULT, DEFAULT) };
        DwmSetWindowAttribute(hwnd, DWMWA_CAPTION_COLOR as u32, &cap as *const _ as _, 4);
        DwmSetWindowAttribute(hwnd, DWMWA_TEXT_COLOR as u32, &txt as *const _ as _, 4);
    }
}

/// ポップアップメニューをダークにする（uxtheme の序数エクスポート。無い環境では何もしない）
pub fn set_app_mode(dark: bool) {
    unsafe {
        let lib = LoadLibraryW(w("uxtheme.dll").as_ptr());
        if lib.is_null() {
            return;
        }
        let Some(set_mode) = GetProcAddress(lib, 135 as _) else { return };
        let set_mode: unsafe extern "system" fn(i32) -> i32 = std::mem::transmute(set_mode);
        set_mode(if dark { 2 } else { 0 });
        if let Some(flush) = GetProcAddress(lib, 136 as _) {
            let flush: unsafe extern "system" fn() = std::mem::transmute(flush);
            flush();
        }
    }
}

pub fn style_child(h: HWND, kind: Kind, dark: bool) {
    unsafe {
        let theme = match (kind, dark) {
            (Kind::Button | Kind::Scintilla, true) => "DarkMode_Explorer",
            (Kind::Edit, true) => "DarkMode_CFD",
            // チェックボックスはテーマを外すと WM_CTLCOLORSTATIC の文字色が効く
            (Kind::CheckBox, true) => "",
            (_, false) => "Explorer",
        };
        SetWindowTheme(h, w(theme).as_ptr(), if theme.is_empty() { w("").as_ptr() } else { std::ptr::null() });
        if kind == Kind::Scintilla {
            // 3D の縁は明るい線が残るのでダーク時は外す
            let ex = GetWindowLongW(h, GWL_EXSTYLE) as u32;
            let new = if dark { ex & !WS_EX_CLIENTEDGE } else { ex | WS_EX_CLIENTEDGE };
            if new != ex {
                SetWindowLongW(h, GWL_EXSTYLE, new as i32);
                SetWindowPos(h, std::ptr::null_mut(), 0, 0, 0, 0, SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE);
            }
        }
    }
}

fn menubar_rect(hwnd: HWND) -> Option<RECT> {
    unsafe {
        let mut mbi: MENUBARINFO = std::mem::zeroed();
        mbi.cbSize = std::mem::size_of::<MENUBARINFO>() as u32;
        if GetMenuBarInfo(hwnd, OBJID_MENU, 0, &mut mbi) == 0 {
            return None;
        }
        let mut win: RECT = std::mem::zeroed();
        GetWindowRect(hwnd, &mut win);
        let r = mbi.rcBar;
        Some(RECT { left: r.left - win.left, top: r.top - win.top, right: r.right - win.left, bottom: r.bottom - win.top })
    }
}

pub fn draw_menubar(hwnd: HWND, lparam: LPARAM, brush: HBRUSH) {
    unsafe {
        let um = &*(lparam as *const UahMenu);
        if let Some(mut rc) = menubar_rect(hwnd) {
            rc.top -= 1;
            FillRect(um.hdc, &rc, brush);
        }
    }
}

pub fn draw_menu_item(lparam: LPARAM, p: &Palette, brush: HBRUSH) {
    unsafe {
        let it = &*(lparam as *const UahDrawMenuItem);
        let mut buf = [0u16; 256];
        let mut mii: MENUITEMINFOW = std::mem::zeroed();
        mii.cbSize = std::mem::size_of::<MENUITEMINFOW>() as u32;
        mii.fMask = MIIM_STRING;
        mii.dwTypeData = buf.as_mut_ptr();
        mii.cch = buf.len() as u32 - 1;
        GetMenuItemInfoW(it.um.hmenu, it.umi.position as u32, 1, &mut mii);
        let state = it.dis.itemState;
        let hot = state & (ODS_HOTLIGHT | ODS_SELECTED) != 0;
        let hdc = it.dis.hDC;
        let mut rc = it.dis.rcItem;
        if hot {
            let hb = CreateSolidBrush(rgb(p.selection) as u32);
            FillRect(hdc, &rc, hb);
            DeleteObject(hb as HGDIOBJ);
        } else {
            FillRect(hdc, &rc, brush);
        }
        let fg = if state & (ODS_INACTIVE | ODS_DISABLED) != 0 { p.margin_fg } else { p.ui_fg };
        SetTextColor(hdc, rgb(fg) as u32);
        SetBkMode(hdc, TRANSPARENT as i32);
        let mut flags = DT_CENTER | DT_SINGLELINE | DT_VCENTER;
        if state & ODS_NOACCEL != 0 {
            flags |= DT_HIDEPREFIX;
        }
        DrawTextW(hdc, buf.as_ptr(), -1, &mut rc, flags);
    }
}

/// システムがメニューバーの下に引く明るい 1px 線を塗り直す（WM_NCPAINT / WM_NCACTIVATE の後に呼ぶ）
pub fn draw_menubar_line(hwnd: HWND, brush: HBRUSH) {
    unsafe {
        let Some(rc) = menubar_rect(hwnd) else { return };
        let line = RECT { left: rc.left, top: rc.bottom, right: rc.right, bottom: rc.bottom + 1 };
        let hdc = GetWindowDC(hwnd);
        FillRect(hdc, &line, brush);
        ReleaseDC(hwnd, hdc);
    }
}

fn text(hdc: HDC, s: &[u16], rc: &mut RECT, flags: u32) {
    unsafe {
        DrawTextW(hdc, s.as_ptr(), -1, rc, flags | DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX);
    }
}

pub fn paint_tabs(hwnd: HWND, p: &Palette, br: &Brushes) {
    unsafe {
        let mut ps: PAINTSTRUCT = std::mem::zeroed();
        let hdc = BeginPaint(hwnd, &mut ps);
        let mut client: RECT = std::mem::zeroed();
        GetClientRect(hwnd, &mut client);
        FillRect(hdc, &client, br.ui);
        let old = SelectObject(hdc, SendMessageW(hwnd, WM_GETFONT, 0, 0) as HGDIOBJ);
        SetBkMode(hdc, TRANSPARENT as i32);
        let n = SendMessageW(hwnd, TCM_GETITEMCOUNT, 0, 0);
        let cur = SendMessageW(hwnd, TCM_GETCURSEL, 0, 0);
        let line = CreateSolidBrush(rgb(p.margin_fg) as u32);
        let mut buf = [0u16; 512];
        for i in 0..n {
            let mut rc: RECT = std::mem::zeroed();
            SendMessageW(hwnd, TCM_GETITEMRECT, i as usize, &mut rc as *mut _ as LPARAM);
            let selected = i == cur;
            if selected {
                FillRect(hdc, &rc, br.edit);
            } else if i + 1 < n {
                let sep = RECT { left: rc.right - 1, top: rc.top + 4, right: rc.right, bottom: rc.bottom - 4 };
                FillRect(hdc, &sep, line);
            }
            let mut item: TCITEMW = std::mem::zeroed();
            item.mask = TCIF_TEXT;
            item.pszText = buf.as_mut_ptr();
            item.cchTextMax = buf.len() as i32;
            SendMessageW(hwnd, TCM_GETITEMW, i as usize, &mut item as *mut _ as LPARAM);
            SetTextColor(hdc, rgb(if selected { p.fg } else { p.ui_fg }) as u32);
            text(hdc, &buf, &mut rc, DT_CENTER);
        }
        DeleteObject(line as HGDIOBJ);
        SelectObject(hdc, old);
        EndPaint(hwnd, &ps);
    }
}

pub fn paint_status(hwnd: HWND, p: &Palette, br: &Brushes) {
    unsafe {
        let mut ps: PAINTSTRUCT = std::mem::zeroed();
        let hdc = BeginPaint(hwnd, &mut ps);
        let mut client: RECT = std::mem::zeroed();
        GetClientRect(hwnd, &mut client);
        FillRect(hdc, &client, br.ui);
        let old = SelectObject(hdc, SendMessageW(hwnd, WM_GETFONT, 0, 0) as HGDIOBJ);
        SetBkMode(hdc, TRANSPARENT as i32);
        SetTextColor(hdc, rgb(p.ui_fg) as u32);
        let n = SendMessageW(hwnd, SB_GETPARTS, 0, 0);
        let line = CreateSolidBrush(rgb(p.margin_fg) as u32);
        let mut buf = [0u16; 1024];
        for i in 0..n {
            let mut rc: RECT = std::mem::zeroed();
            SendMessageW(hwnd, SB_GETRECT, i as usize, &mut rc as *mut _ as LPARAM);
            if i > 0 {
                let sep = RECT { left: rc.left, top: rc.top + 2, right: rc.left + 1, bottom: rc.bottom - 2 };
                FillRect(hdc, &sep, line);
            }
            buf.fill(0);
            SendMessageW(hwnd, SB_GETTEXTW, i as usize, buf.as_mut_ptr() as LPARAM);
            rc.left += 4;
            rc.right -= 2;
            text(hdc, &buf, &mut rc, DT_LEFT | DT_END_ELLIPSIS);
        }
        DeleteObject(line as HGDIOBJ);
        SelectObject(hdc, old);
        EndPaint(hwnd, &ps);
    }
}
