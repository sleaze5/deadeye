use std::sync::atomic::{AtomicU32, Ordering};
use std::{mem, ptr};

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::UI::Shell::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use windows_sys::w;

use crate::{overlay, settings, win};

const WM_TRAY: u32 = WM_APP + 1;
const ID_SETTINGS: usize = 1;
const ID_TOGGLE: usize = 2;
const ID_EXIT: usize = 3;

static TASKBAR_CREATED: AtomicU32 = AtomicU32::new(0);

pub fn create() {
	let class = w!("DeadeyeTray");
	win::register_class(class, wnd_proc);
	unsafe {
		TASKBAR_CREATED
			.store(RegisterWindowMessageW(w!("TaskbarCreated")), Ordering::Relaxed);
	}
	let hwnd = unsafe {
		CreateWindowExW(
			0,
			class,
			w!("Deadeye"),
			WS_OVERLAPPED,
			0,
			0,
			0,
			0,
			ptr::null_mut(),
			ptr::null_mut(),
			win::instance(),
			ptr::null(),
		)
	};
	if hwnd.is_null() {
		win::fail("Failed to create tray window.");
	}
	add_icon(hwnd);
}

unsafe extern "system" fn wnd_proc(
	hwnd: HWND,
	msg: u32,
	wparam: WPARAM,
	lparam: LPARAM,
) -> LRESULT {
	match msg {
		WM_TRAY => on_tray(hwnd, lparam as u32),
		WM_COMMAND => on_command(hwnd, wparam & 0xFFFF),
		WM_DESTROY => unsafe {
			Shell_NotifyIconW(NIM_DELETE, &icon_data(hwnd));
			PostQuitMessage(0);
		},
		m if m == TASKBAR_CREATED.load(Ordering::Relaxed) => add_icon(hwnd),
		_ => return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
	}
	0
}

fn on_tray(hwnd: HWND, event: u32) {
	match event {
		WM_LBUTTONUP => settings::open(),
		WM_RBUTTONUP => show_menu(hwnd),
		_ => {},
	}
}

fn on_command(hwnd: HWND, id: usize) {
	match id {
		ID_SETTINGS => settings::open(),
		ID_TOGGLE => overlay::toggle(),
		ID_EXIT => {
			settings::close();
			unsafe {
				DestroyWindow(hwnd);
			}
		},
		_ => {},
	}
}

fn show_menu(hwnd: HWND) {
	let check = if overlay::is_visible() { MF_CHECKED } else { MF_UNCHECKED };
	unsafe {
		let menu = CreatePopupMenu();
		AppendMenuW(menu, MF_STRING, ID_SETTINGS, w!("Settings…"));
		AppendMenuW(menu, MF_STRING | check, ID_TOGGLE, w!("Show crosshair"));
		AppendMenuW(menu, MF_SEPARATOR, 0, ptr::null());
		AppendMenuW(menu, MF_STRING, ID_EXIT, w!("Exit"));
		SetMenuDefaultItem(menu, ID_SETTINGS as u32, 0);

		let mut pt: POINT = mem::zeroed();
		GetCursorPos(&mut pt);
		SetForegroundWindow(hwnd);
		TrackPopupMenu(menu, TPM_RIGHTBUTTON, pt.x, pt.y, 0, hwnd, ptr::null());
		PostMessageW(hwnd, WM_NULL, 0, 0);
		DestroyMenu(menu);
	}
}

fn add_icon(hwnd: HWND) {
	unsafe {
		Shell_NotifyIconW(NIM_ADD, &icon_data(hwnd));
	}
}

fn icon_data(hwnd: HWND) -> NOTIFYICONDATAW {
	let mut nid: NOTIFYICONDATAW = unsafe { mem::zeroed() };
	nid.cbSize = mem::size_of::<NOTIFYICONDATAW>() as u32;
	nid.hWnd = hwnd;
	nid.uID = 1;
	nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
	nid.uCallbackMessage = WM_TRAY;
	nid.hIcon = win::icon(unsafe { GetSystemMetrics(SM_CXSMICON) });
	for (dst, src) in nid.szTip.iter_mut().zip("Deadeye".encode_utf16()) {
		*dst = src;
	}
	nid
}
