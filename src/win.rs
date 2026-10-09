use std::{mem, ptr};

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::{BI_RGB, BITMAPINFO, BITMAPINFOHEADER};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use windows_sys::core::PCWSTR;
use windows_sys::w;

pub type WndProc = unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT;

pub fn wide(text: &str) -> Vec<u16> {
	text.encode_utf16().chain(Some(0)).collect()
}

pub fn instance() -> HINSTANCE {
	unsafe { GetModuleHandleW(ptr::null()) }
}

pub fn icon(size: i32) -> HICON {
	unsafe { LoadImageW(instance(), 1 as PCWSTR, IMAGE_ICON, size, size, LR_SHARED) }
}

pub fn bitmap_info(width: i32, height: i32) -> BITMAPINFO {
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

pub fn register_class(name: PCWSTR, wnd_proc: WndProc) {
	let class = WNDCLASSW {
		lpfnWndProc: Some(wnd_proc),
		hInstance: instance(),
		lpszClassName: name,
		..unsafe { mem::zeroed() }
	};
	if unsafe { RegisterClassW(&class) } == 0 {
		fail("Failed to register window class.");
	}
}

pub fn show_error(owner: HWND, message: &str) {
	unsafe {
		MessageBoxW(owner, wide(message).as_ptr(), w!("Deadeye"), MB_ICONERROR);
	}
}

pub fn confirm(owner: HWND, message: &str) -> bool {
	let answer = unsafe {
		MessageBoxW(
			owner,
			wide(message).as_ptr(),
			w!("Deadeye"),
			MB_YESNO | MB_ICONQUESTION,
		)
	};
	answer == IDYES
}

pub fn fail(message: &str) -> ! {
	show_error(ptr::null_mut(), message);
	std::process::exit(1);
}
