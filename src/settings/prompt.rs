use std::cell::RefCell;

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::UI::Controls::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use windows_sys::core::PCWSTR;

use super::controls::{control_text, set_control_text};
use super::ids::{ID_PROFILE_NAME_EDIT, IDD_PROFILE_NAME_PROMPT};
use crate::profiles::MAX_PROFILE_NAME_LENGTH;
use crate::win;

const NAME_ACCEPTED: isize = 1;
const NAME_CANCELLED: isize = 0;

thread_local! {
	static ENTERED_PROFILE_NAME: RefCell<String> = const { RefCell::new(String::new()) };
}

pub fn ask_profile_name(owner: HWND, initial_name: &str) -> Option<String> {
	ENTERED_PROFILE_NAME.set(initial_name.into());
	let dialog_result = unsafe {
		DialogBoxParamW(
			win::module_instance(),
			IDD_PROFILE_NAME_PROMPT as PCWSTR,
			owner,
			Some(dialog_proc),
			0,
		)
	};
	(dialog_result == NAME_ACCEPTED).then(|| ENTERED_PROFILE_NAME.take())
}

unsafe extern "system" fn dialog_proc(
	dialog: HWND,
	message: u32,
	wparam: WPARAM,
	_: LPARAM,
) -> isize {
	let control_id = (wparam & 0xFFFF) as i32;
	match message {
		WM_INITDIALOG => init_name_field(dialog),
		WM_COMMAND if control_id == IDOK => {
			ENTERED_PROFILE_NAME.set(control_text(dialog, ID_PROFILE_NAME_EDIT));
			unsafe { EndDialog(dialog, NAME_ACCEPTED) };
		},
		WM_COMMAND if control_id == IDCANCEL => unsafe {
			EndDialog(dialog, NAME_CANCELLED);
		},
		_ => return 0,
	}
	1
}

fn init_name_field(dialog: HWND) {
	ENTERED_PROFILE_NAME.with_borrow(|entered_name| {
		set_control_text(dialog, ID_PROFILE_NAME_EDIT, entered_name)
	});
	unsafe {
		SendDlgItemMessageW(
			dialog,
			ID_PROFILE_NAME_EDIT,
			EM_SETLIMITTEXT,
			MAX_PROFILE_NAME_LENGTH,
			0,
		);
		SendDlgItemMessageW(dialog, ID_PROFILE_NAME_EDIT, EM_SETSEL, 0, -1);
	}
}
