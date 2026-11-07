use std::{mem, ptr};

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::{BI_RGB, BITMAPINFO, BITMAPINFOHEADER};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use windows_sys::core::PCWSTR;
use windows_sys::w;

const APP_TITLE: PCWSTR = w!("Deadeye");
const APP_ICON_RESOURCE_ID: usize = 1;

type WindowProcedure = unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT;

pub fn null_terminated_utf16(text: &str) -> Vec<u16> {
	text.encode_utf16().chain(Some(0)).collect()
}

pub fn module_instance() -> HINSTANCE {
	unsafe { GetModuleHandleW(ptr::null()) }
}

pub fn load_app_icon(size_in_pixels: i32) -> HICON {
	unsafe {
		LoadImageW(
			module_instance(),
			APP_ICON_RESOURCE_ID as PCWSTR,
			IMAGE_ICON,
			size_in_pixels,
			size_in_pixels,
			LR_SHARED,
		)
	}
}

pub fn top_down_32_bit_bitmap_info(width: i32, height: i32) -> BITMAPINFO {
	BITMAPINFO {
		bmiHeader: BITMAPINFOHEADER {
			biSize: mem::size_of::<BITMAPINFOHEADER>() as u32,
			biWidth: width,
			biHeight: -height,
			biPlanes: 1,
			biBitCount: 32,
			biCompression: BI_RGB,
			..unsafe { mem::zeroed() }
		},
		..unsafe { mem::zeroed() }
	}
}

pub fn create_window(
	window_description: &str,
	class_name: PCWSTR,
	window_procedure: WindowProcedure,
	style: WINDOW_STYLE,
	ex_style: WINDOW_EX_STYLE,
) -> HWND {
	let window_class = WNDCLASSW {
		lpfnWndProc: Some(window_procedure),
		hInstance: module_instance(),
		lpszClassName: class_name,
		..unsafe { mem::zeroed() }
	};
	if unsafe { RegisterClassW(&window_class) } == 0 {
		exit_with_error("Failed to register window class.");
	}
	let window = unsafe {
		CreateWindowExW(
			ex_style,
			class_name,
			APP_TITLE,
			style,
			0,
			0,
			0,
			0,
			ptr::null_mut(),
			ptr::null_mut(),
			module_instance(),
			ptr::null(),
		)
	};
	if window.is_null() {
		exit_with_error(&format!("Failed to create {window_description} window."));
	}
	window
}

pub fn show_error(owner: HWND, message: &str) {
	unsafe {
		MessageBoxW(
			owner,
			null_terminated_utf16(message).as_ptr(),
			APP_TITLE,
			MB_ICONERROR,
		);
	}
}

pub fn ask_yes_no(owner: HWND, message: &str) -> bool {
	let clicked_button = unsafe {
		MessageBoxW(
			owner,
			null_terminated_utf16(message).as_ptr(),
			APP_TITLE,
			MB_YESNO | MB_ICONQUESTION,
		)
	};
	clicked_button == IDYES
}

pub fn exit_with_error(message: &str) -> ! {
	show_error(ptr::null_mut(), message);
	std::process::exit(1);
}
