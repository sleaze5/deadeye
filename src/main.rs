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
	let controls = INITCOMMONCONTROLSEX {
		dwSize: mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
		dwICC: ICC_BAR_CLASSES | ICC_UPDOWN_CLASS,
	};
	unsafe {
		InitCommonControlsEx(&controls);
	}

	profiles::load();
	overlay::create(profiles::saved(), profiles::windows_scaling());
	tray::create();
	run();
}

fn run() {
	unsafe {
		let mut msg: MSG = mem::zeroed();
		while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) > 0 {
			let dialog = settings::dialog();
			if !dialog.is_null() && IsDialogMessageW(dialog, &msg) != 0 {
				continue;
			}
			TranslateMessage(&msg);
			DispatchMessageW(&msg);
		}
	}
}
