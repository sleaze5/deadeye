use std::sync::atomic::{AtomicU32, Ordering};
use std::{mem, ptr};

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::UI::Shell::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use windows_sys::w;

use crate::{overlay, settings, win};

const WM_TRAY_ICON: u32 = WM_APP + 1;
const ID_MENU_OPEN_SETTINGS: usize = 1;
const ID_MENU_TOGGLE_CROSSHAIR: usize = 2;
const ID_MENU_EXIT: usize = 3;

static WM_TASKBAR_CREATED: AtomicU32 = AtomicU32::new(0);

pub fn create() {
	unsafe {
		WM_TASKBAR_CREATED
			.store(RegisterWindowMessageW(w!("TaskbarCreated")), Ordering::Relaxed);
	}
	let tray_window =
		win::create_window("tray", w!("DeadeyeTray"), wnd_proc, WS_OVERLAPPED, 0);
	add_tray_icon(tray_window);
}

unsafe extern "system" fn wnd_proc(
	tray_window: HWND,
	message: u32,
	wparam: WPARAM,
	lparam: LPARAM,
) -> LRESULT {
	match message {
		WM_TRAY_ICON => on_tray_icon_event(tray_window, lparam as u32),
		WM_COMMAND => on_menu_command(tray_window, wparam & 0xFFFF),
		WM_DESTROY => unsafe {
			Shell_NotifyIconW(NIM_DELETE, &tray_icon_data(tray_window));
			PostQuitMessage(0);
		},
		registered_message
			if registered_message == WM_TASKBAR_CREATED.load(Ordering::Relaxed) =>
		{
			add_tray_icon(tray_window)
		},
		_ => return unsafe { DefWindowProcW(tray_window, message, wparam, lparam) },
	}
	0
}

fn on_tray_icon_event(tray_window: HWND, mouse_message: u32) {
	match mouse_message {
		WM_LBUTTONUP => settings::open(),
		WM_RBUTTONUP => show_context_menu(tray_window),
		_ => {},
	}
}

fn on_menu_command(tray_window: HWND, menu_item_id: usize) {
	match menu_item_id {
		ID_MENU_OPEN_SETTINGS => settings::open(),
		ID_MENU_TOGGLE_CROSSHAIR => overlay::toggle_visibility(),
		ID_MENU_EXIT => {
			settings::close();
			unsafe {
				DestroyWindow(tray_window);
			}
		},
		_ => {},
	}
}

fn show_context_menu(tray_window: HWND) {
	let crosshair_check_state =
		if overlay::is_visible() { MF_CHECKED } else { MF_UNCHECKED };
	unsafe {
		let context_menu = CreatePopupMenu();
		AppendMenuW(context_menu, MF_STRING, ID_MENU_OPEN_SETTINGS, w!("Settings…"));
		AppendMenuW(
			context_menu,
			MF_STRING | crosshair_check_state,
			ID_MENU_TOGGLE_CROSSHAIR,
			w!("Show crosshair"),
		);
		AppendMenuW(context_menu, MF_SEPARATOR, 0, ptr::null());
		AppendMenuW(context_menu, MF_STRING, ID_MENU_EXIT, w!("Exit"));
		SetMenuDefaultItem(context_menu, ID_MENU_OPEN_SETTINGS as u32, 0);

		let mut cursor_position: POINT = mem::zeroed();
		GetCursorPos(&mut cursor_position);
		SetForegroundWindow(tray_window);
		TrackPopupMenu(
			context_menu,
			TPM_RIGHTBUTTON,
			cursor_position.x,
			cursor_position.y,
			0,
			tray_window,
			ptr::null(),
		);
		PostMessageW(tray_window, WM_NULL, 0, 0);
		DestroyMenu(context_menu);
	}
}

fn add_tray_icon(tray_window: HWND) {
	unsafe {
		Shell_NotifyIconW(NIM_ADD, &tray_icon_data(tray_window));
	}
}

fn tray_icon_data(tray_window: HWND) -> NOTIFYICONDATAW {
	let mut icon_data: NOTIFYICONDATAW = unsafe { mem::zeroed() };
	icon_data.cbSize = mem::size_of::<NOTIFYICONDATAW>() as u32;
	icon_data.hWnd = tray_window;
	icon_data.uID = 1;
	icon_data.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
	icon_data.uCallbackMessage = WM_TRAY_ICON;
	icon_data.hIcon = win::load_app_icon(unsafe { GetSystemMetrics(SM_CXSMICON) });
	for (tip_character, title_character) in
		icon_data.szTip.iter_mut().zip("Deadeye".encode_utf16())
	{
		*tip_character = title_character;
	}
	icon_data
}
