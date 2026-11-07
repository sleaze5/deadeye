use std::str::FromStr;

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::UI::Controls::*;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use crate::win;

const TBM_GETPOS: u32 = WM_USER;
const OPACITY_STEPS: f32 = 100.0;

pub fn set_enabled(
	dialog: HWND,
	control_ids: impl IntoIterator<Item = i32>,
	is_enabled: bool,
) {
	for control_id in control_ids {
		unsafe {
			EnableWindow(GetDlgItem(dialog, control_id), is_enabled.into());
		}
	}
}

pub fn is_checked(dialog: HWND, control_id: i32) -> bool {
	unsafe { IsDlgButtonChecked(dialog, control_id) == BST_CHECKED }
}

pub fn set_checked(dialog: HWND, control_id: i32, is_checked: bool) {
	let check_state = if is_checked { BST_CHECKED } else { BST_UNCHECKED };
	unsafe {
		CheckDlgButton(dialog, control_id, check_state);
	}
}

pub fn init_opacity_range(dialog: HWND, control_id: i32) {
	unsafe {
		SendDlgItemMessageW(
			dialog,
			control_id,
			TBM_SETRANGEMAX,
			0,
			OPACITY_STEPS as isize,
		);
	}
}

pub fn opacity(dialog: HWND, control_id: i32) -> f32 {
	unsafe {
		SendDlgItemMessageW(dialog, control_id, TBM_GETPOS, 0, 0) as f32 / OPACITY_STEPS
	}
}

pub fn set_opacity(dialog: HWND, control_id: i32, opacity: f32) {
	unsafe {
		SendDlgItemMessageW(
			dialog,
			control_id,
			TBM_SETPOS,
			1,
			(opacity * OPACITY_STEPS).round() as isize,
		);
	}
}

pub fn parse_text_or<T: FromStr>(dialog: HWND, control_id: i32, fallback: T) -> T {
	control_text(dialog, control_id).trim().parse().unwrap_or(fallback)
}

pub fn control_text(dialog: HWND, control_id: i32) -> String {
	let mut text_buffer = [0u16; 64];
	let text_length = unsafe {
		GetDlgItemTextW(
			dialog,
			control_id,
			text_buffer.as_mut_ptr(),
			text_buffer.len() as i32,
		)
	};
	String::from_utf16_lossy(&text_buffer[..text_length as usize])
}

pub fn set_control_text(dialog: HWND, control_id: i32, new_text: &str) {
	if control_text(dialog, control_id) != new_text {
		unsafe {
			SetDlgItemTextW(
				dialog,
				control_id,
				win::null_terminated_utf16(new_text).as_ptr(),
			);
		}
	}
}
