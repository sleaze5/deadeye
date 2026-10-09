use std::cell::Cell;
use std::ptr;

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::UI::HiDpi::GetDpiForWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use windows_sys::w;

use crate::config::Crosshair;
use crate::{render, win};

thread_local! {
	static WINDOW: Cell<HWND> = const { Cell::new(ptr::null_mut()) };
	static CROSSHAIR: Cell<Crosshair> = Cell::new(Crosshair::default());
	static WINDOWS_SCALING: Cell<bool> = const { Cell::new(true) };
}

pub fn create(crosshair: Crosshair, windows_scaling: bool) {
	let class = w!("DeadeyeOverlay");
	win::register_class(class, wnd_proc);
	let hwnd = unsafe {
		CreateWindowExW(
			WS_EX_LAYERED
				| WS_EX_TRANSPARENT
				| WS_EX_TOPMOST
				| WS_EX_TOOLWINDOW
				| WS_EX_NOACTIVATE,
			class,
			w!("Deadeye"),
			WS_POPUP,
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
		win::fail("Failed to create overlay window.");
	}
	WINDOW.set(hwnd);
	WINDOWS_SCALING.set(windows_scaling);
	apply(crosshair);
	unsafe {
		ShowWindow(hwnd, SW_SHOWNOACTIVATE);
	}
}

pub fn crosshair() -> Crosshair {
	CROSSHAIR.get()
}

pub fn apply(crosshair: Crosshair) {
	CROSSHAIR.set(crosshair);
	present(WINDOW.get());
}

pub fn set_windows_scaling(on: bool) {
	WINDOWS_SCALING.set(on);
	present(WINDOW.get());
}

pub fn scale() -> f32 {
	if WINDOWS_SCALING.get() {
		unsafe { GetDpiForWindow(WINDOW.get()) as f32 / 96.0 }
	} else {
		1.0
	}
}

pub fn is_visible() -> bool {
	unsafe { IsWindowVisible(WINDOW.get()) != 0 }
}

pub fn toggle() {
	let cmd = if is_visible() { SW_HIDE } else { SW_SHOWNOACTIVATE };
	unsafe {
		ShowWindow(WINDOW.get(), cmd);
	}
}

fn present(hwnd: HWND) {
	let image = render::render(&CROSSHAIR.get(), scale());
	let info = win::bitmap_info(image.size, image.size);
	unsafe {
		let screen = GetDC(ptr::null_mut());
		let memory = CreateCompatibleDC(screen);
		let mut bits = ptr::null_mut();
		let bitmap = CreateDIBSection(
			screen,
			&info,
			DIB_RGB_COLORS,
			&mut bits,
			ptr::null_mut(),
			0,
		);
		if bitmap.is_null() {
			DeleteDC(memory);
			ReleaseDC(ptr::null_mut(), screen);
			win::show_error(ptr::null_mut(), "Failed to draw the crosshair.");
			return;
		}
		ptr::copy_nonoverlapping(
			image.pixels.as_ptr(),
			bits.cast(),
			image.pixels.len(),
		);
		let previous = SelectObject(memory, bitmap);

		let position = POINT {
			x: GetSystemMetrics(SM_CXSCREEN) / 2 - image.size / 2,
			y: GetSystemMetrics(SM_CYSCREEN) / 2 - image.size / 2,
		};
		let size = SIZE { cx: image.size, cy: image.size };
		let source = POINT { x: 0, y: 0 };
		let blend = BLENDFUNCTION {
			BlendOp: AC_SRC_OVER as u8,
			BlendFlags: 0,
			SourceConstantAlpha: 255,
			AlphaFormat: AC_SRC_ALPHA as u8,
		};
		UpdateLayeredWindow(
			hwnd, screen, &position, &size, memory, &source, 0, &blend, ULW_ALPHA,
		);

		SelectObject(memory, previous);
		DeleteObject(bitmap);
		DeleteDC(memory);
		ReleaseDC(ptr::null_mut(), screen);
	}
}

unsafe extern "system" fn wnd_proc(
	hwnd: HWND,
	msg: u32,
	wparam: WPARAM,
	lparam: LPARAM,
) -> LRESULT {
	match msg {
		WM_DISPLAYCHANGE | WM_DPICHANGED => present(hwnd),
		_ => return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
	}
	0
}
