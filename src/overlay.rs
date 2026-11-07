use std::cell::Cell;
use std::ptr;

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::UI::HiDpi::GetDpiForWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use windows_sys::w;

use crate::config::Crosshair;
use crate::{profiles, render, win};

const DEFAULT_DPI: f32 = 96.0;

thread_local! {
	static OVERLAY_WINDOW: Cell<HWND> = const { Cell::new(ptr::null_mut()) };
	static DISPLAYED_CROSSHAIR: Cell<Crosshair> = Cell::new(Crosshair::default());
}

pub fn create(crosshair: Crosshair) {
	let overlay_window = win::create_window(
		"overlay",
		w!("DeadeyeOverlay"),
		wnd_proc,
		WS_POPUP,
		WS_EX_LAYERED
			| WS_EX_TRANSPARENT
			| WS_EX_TOPMOST
			| WS_EX_TOOLWINDOW
			| WS_EX_NOACTIVATE,
	);
	OVERLAY_WINDOW.set(overlay_window);
	display(crosshair);
	unsafe {
		ShowWindow(overlay_window, SW_SHOWNOACTIVATE);
	}
}

pub fn displayed_crosshair() -> Crosshair {
	DISPLAYED_CROSSHAIR.get()
}

pub fn display(crosshair: Crosshair) {
	DISPLAYED_CROSSHAIR.set(crosshair);
	draw_to_window(OVERLAY_WINDOW.get());
}

pub fn redraw() {
	draw_to_window(OVERLAY_WINDOW.get());
}

pub fn scale_factor() -> f32 {
	if profiles::uses_windows_scaling() {
		unsafe { GetDpiForWindow(OVERLAY_WINDOW.get()) as f32 / DEFAULT_DPI }
	} else {
		1.0
	}
}

pub fn is_visible() -> bool {
	unsafe { IsWindowVisible(OVERLAY_WINDOW.get()) != 0 }
}

pub fn toggle_visibility() {
	let show_command = if is_visible() { SW_HIDE } else { SW_SHOWNOACTIVATE };
	unsafe {
		ShowWindow(OVERLAY_WINDOW.get(), show_command);
	}
}

fn draw_to_window(overlay_window: HWND) {
	let crosshair_image =
		render::render_crosshair(&DISPLAYED_CROSSHAIR.get(), scale_factor());
	let bitmap_info = win::top_down_32_bit_bitmap_info(
		crosshair_image.side_length,
		crosshair_image.side_length,
	);
	unsafe {
		let screen_device_context = GetDC(ptr::null_mut());
		let memory_device_context = CreateCompatibleDC(screen_device_context);
		let mut bitmap_pixels = ptr::null_mut();
		let bitmap = CreateDIBSection(
			screen_device_context,
			&bitmap_info,
			DIB_RGB_COLORS,
			&mut bitmap_pixels,
			ptr::null_mut(),
			0,
		);
		if bitmap.is_null() {
			DeleteDC(memory_device_context);
			ReleaseDC(ptr::null_mut(), screen_device_context);
			win::show_error(ptr::null_mut(), "Failed to draw the crosshair.");
			return;
		}
		ptr::copy_nonoverlapping(
			crosshair_image.pixels.as_ptr(),
			bitmap_pixels.cast(),
			crosshair_image.pixels.len(),
		);
		let previously_selected_bitmap = SelectObject(memory_device_context, bitmap);

		let window_top_left = POINT {
			x: GetSystemMetrics(SM_CXSCREEN) / 2 - crosshair_image.side_length / 2,
			y: GetSystemMetrics(SM_CYSCREEN) / 2 - crosshair_image.side_length / 2,
		};
		let window_size =
			SIZE { cx: crosshair_image.side_length, cy: crosshair_image.side_length };
		let source_top_left = POINT { x: 0, y: 0 };
		let blend_function = BLENDFUNCTION {
			BlendOp: AC_SRC_OVER as u8,
			BlendFlags: 0,
			SourceConstantAlpha: 255,
			AlphaFormat: AC_SRC_ALPHA as u8,
		};
		UpdateLayeredWindow(
			overlay_window,
			screen_device_context,
			&window_top_left,
			&window_size,
			memory_device_context,
			&source_top_left,
			0,
			&blend_function,
			ULW_ALPHA,
		);

		SelectObject(memory_device_context, previously_selected_bitmap);
		DeleteObject(bitmap);
		DeleteDC(memory_device_context);
		ReleaseDC(ptr::null_mut(), screen_device_context);
	}
}

unsafe extern "system" fn wnd_proc(
	hwnd: HWND,
	message: u32,
	wparam: WPARAM,
	lparam: LPARAM,
) -> LRESULT {
	match message {
		WM_DISPLAYCHANGE | WM_DPICHANGED => draw_to_window(hwnd),
		_ => return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
	}
	0
}
