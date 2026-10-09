use std::cell::RefCell;

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::UI::Controls::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use windows_sys::core::PCWSTR;

use super::controls::{set_text, text};
use super::ids::{ID_NAME, IDD_NAME};
use crate::profiles::MAX_NAME_LENGTH;
use crate::win;

thread_local! {
	static NAME: RefCell<String> = const { RefCell::new(String::new()) };
}

pub fn ask_name(owner: HWND, initial: &str) -> Option<String> {
	NAME.set(initial.into());
	let result = unsafe {
		DialogBoxParamW(
			win::instance(),
			IDD_NAME as PCWSTR,
			owner,
			Some(dialog_proc),
			0,
		)
	};
	(result == 1).then(|| NAME.take())
}

unsafe extern "system" fn dialog_proc(
	dialog: HWND,
	msg: u32,
	wparam: WPARAM,
	_: LPARAM,
) -> isize {
	let id = (wparam & 0xFFFF) as i32;
	match msg {
		WM_INITDIALOG => init(dialog),
		WM_COMMAND if id == IDOK => {
			NAME.set(text(dialog, ID_NAME));
			unsafe { EndDialog(dialog, 1) };
		},
		WM_COMMAND if id == IDCANCEL => unsafe {
			EndDialog(dialog, 0);
		},
		_ => return 0,
	}
	1
}

fn init(dialog: HWND) {
	NAME.with_borrow(|name| set_text(dialog, ID_NAME, name));
	unsafe {
		SendDlgItemMessageW(dialog, ID_NAME, EM_SETLIMITTEXT, MAX_NAME_LENGTH, 0);
		SendDlgItemMessageW(dialog, ID_NAME, EM_SETSEL, 0, -1);
	}
}
