//! 子コントロール生成などの UI ヘルパ

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::Graphics::Gdi::{
    CLEARTYPE_QUALITY, CreateFontW, DEFAULT_CHARSET, FW_NORMAL, HFONT,
};
use windows_sys::Win32::UI::Controls::{BST_CHECKED, BST_UNCHECKED};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BM_GETCHECK, BM_SETCHECK, CreateWindowExW, HMENU, MoveWindow, SendMessageW, ShowWindow,
    SW_HIDE, SW_SHOW, WINDOW_EX_STYLE, WINDOW_STYLE, WM_SETFONT, WS_CHILD,
};

use crate::util::w;

pub const SS_LEFT: u32 = 0;
pub const SS_CENTER: u32 = 1;
pub const SS_NOTIFY: u32 = 0x100;

pub fn control(
    parent: HWND,
    hinst: *mut core::ffi::c_void,
    ex: WINDOW_EX_STYLE,
    class: &str,
    text: &str,
    style: WINDOW_STYLE,
    id: usize,
    font: HFONT,
) -> HWND {
    unsafe {
        let h = CreateWindowExW(
            ex,
            w(class).as_ptr(),
            w(text).as_ptr(),
            WS_CHILD | style,
            0,
            0,
            10,
            10,
            parent,
            id as HMENU,
            hinst,
            std::ptr::null(),
        );
        SendMessageW(h, WM_SETFONT, font as usize, 1);
        h
    }
}

pub fn ui_font(dpi: u32) -> HFONT {
    let h = -((9 * dpi as i32) / 72);
    unsafe {
        CreateFontW(
            h, 0, 0, 0, FW_NORMAL as i32, 0, 0, 0, DEFAULT_CHARSET as u32, 0, 0,
            CLEARTYPE_QUALITY as u32, 0, w("Yu Gothic UI").as_ptr(),
        )
    }
}

pub fn set_font(h: HWND, font: HFONT) {
    unsafe {
        SendMessageW(h, WM_SETFONT, font as usize, 1);
    }
}

pub fn is_checked(h: HWND) -> bool {
    unsafe { SendMessageW(h, BM_GETCHECK, 0, 0) == BST_CHECKED as isize }
}

pub fn set_checked(h: HWND, on: bool) {
    unsafe {
        SendMessageW(h, BM_SETCHECK, if on { BST_CHECKED } else { BST_UNCHECKED } as usize, 0);
    }
}

pub fn place(h: HWND, x: i32, y: i32, cx: i32, cy: i32) {
    unsafe {
        MoveWindow(h, x, y, cx.max(0), cy.max(0), 1);
    }
}

pub fn show(h: HWND, on: bool) {
    unsafe {
        ShowWindow(h, if on { SW_SHOW } else { SW_HIDE });
    }
}

#[inline]
pub fn scale(v: i32, dpi: u32) -> i32 {
    v * dpi as i32 / 96
}
