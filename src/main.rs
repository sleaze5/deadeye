#![windows_subsystem = "windows"]

mod config;
mod overlay;
mod profiles;
mod render;
mod settings;
mod tray;
mod win;

use std::{mem, ptr};

use windows_sys::Win32::UI::Controls::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

fn main() {
	let common_control_classes = INITCOMMONCONTROLSEX {
		dwSize: mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
		dwICC: ICC_BAR_CLASSES | ICC_UPDOWN_CLASS,
	};
	unsafe {
		InitCommonControlsEx(&common_control_classes);
	}

	profiles::load_from_disk();
	overlay::create(profiles::saved_crosshair());
	tray::create();
	run_message_loop();
}

fn run_message_loop() {
	unsafe {
		let mut message: MSG = mem::zeroed();
		while GetMessageW(&mut message, ptr::null_mut(), 0, 0) > 0 {
			let settings_dialog = settings::dialog_window();
			if !settings_dialog.is_null()
				&& IsDialogMessageW(settings_dialog, &message) != 0
			{
				continue;
			}
			TranslateMessage(&message);
			DispatchMessageW(&message);
		}
	}
}
