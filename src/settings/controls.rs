use std::str::FromStr;

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::UI::Controls::*;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use crate::win;

const TBM_GETPOS: u32 = WM_USER;
const OPACITY_STEPS: f32 = 100.0;

pub fn enable(dialog: HWND, ids: impl IntoIterator<Item = i32>, on: bool) {
	for id in ids {
		unsafe {
			EnableWindow(GetDlgItem(dialog, id), on.into());
		}
	}
}

pub fn checked(dialog: HWND, id: i32) -> bool {
	unsafe { IsDlgButtonChecked(dialog, id) == BST_CHECKED }
}

pub fn set_checked(dialog: HWND, id: i32, on: bool) {
	let state = if on { BST_CHECKED } else { BST_UNCHECKED };
	unsafe {
		CheckDlgButton(dialog, id, state);
	}
}

pub fn init_opacity(dialog: HWND, id: i32) {
	unsafe {
		SendDlgItemMessageW(dialog, id, TBM_SETRANGEMAX, 0, OPACITY_STEPS as isize);
	}
}

pub fn opacity(dialog: HWND, id: i32) -> f32 {
	unsafe { SendDlgItemMessageW(dialog, id, TBM_GETPOS, 0, 0) as f32 / OPACITY_STEPS }
}

pub fn set_opacity(dialog: HWND, id: i32, value: f32) {
	unsafe {
		SendDlgItemMessageW(
			dialog,
			id,
			TBM_SETPOS,
			1,
			(value * OPACITY_STEPS).round() as isize,
		);
	}
}

pub fn parse<T: FromStr>(dialog: HWND, id: i32, fallback: T) -> T {
	text(dialog, id).trim().parse().unwrap_or(fallback)
}

pub fn text(dialog: HWND, id: i32) -> String {
	let mut buffer = [0u16; 64];
	let len = unsafe {
		GetDlgItemTextW(dialog, id, buffer.as_mut_ptr(), buffer.len() as i32)
	};
	String::from_utf16_lossy(&buffer[..len as usize])
}

pub fn set_text(dialog: HWND, id: i32, value: &str) {
	if text(dialog, id) != value {
		unsafe {
			SetDlgItemTextW(dialog, id, win::wide(value).as_ptr());
		}
	}
}
