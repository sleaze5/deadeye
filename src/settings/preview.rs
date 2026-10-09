use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::UI::Controls::DRAWITEMSTRUCT;

use crate::{overlay, render, win};

const BACKGROUND: u32 = 0x00404040;
const MAX_ZOOM: i32 = 16;

pub fn draw(item: &DRAWITEMSTRUCT) {
	let image = render::render(&overlay::crosshair(), overlay::scale());
	let rect = item.rcItem;
	let (width, height) = (rect.right - rect.left, rect.bottom - rect.top);
	let side = width.min(height);
	let zoom = (side / (image.size + 4)).clamp(1, MAX_ZOOM);
	let left = (width - image.size * zoom) / 2;
	let top = (height - image.size * zoom) / 2;
	let source = 0..image.size;
	let square_x = (width - side) / 2..(width + side) / 2;
	let square_y = (height - side) / 2..(height + side) / 2;
	let [r, g, b, _] = unsafe { GetSysColor(COLOR_BTNFACE) }.to_le_bytes();
	let face = u32::from_be_bytes([0, r, g, b]);

	let mut frame = Vec::with_capacity((width * height) as usize);
	for y in 0..height {
		let row = (y - top).div_euclid(zoom);
		for x in 0..width {
			let column = (x - left).div_euclid(zoom);
			let pixel = if !square_x.contains(&x) || !square_y.contains(&y) {
				face
			} else if source.contains(&row) && source.contains(&column) {
				over_background(image.pixels[(row * image.size + column) as usize])
			} else {
				BACKGROUND
			};
			frame.push(pixel);
		}
	}
	let info = win::bitmap_info(width, height);
	unsafe {
		SetDIBitsToDevice(
			item.hDC,
			rect.left,
			rect.top,
			width as u32,
			height as u32,
			0,
			0,
			0,
			height as u32,
			frame.as_ptr().cast(),
			&info,
			DIB_RGB_COLORS,
		);
	}
}

fn over_background(pixel: u32) -> u32 {
	let rest = 255 - (pixel >> 24);
	let channel = |shift: u32| {
		let top = (pixel >> shift) & 0xFF;
		let below = (BACKGROUND >> shift) & 0xFF;
		(top + below * rest / 255) << shift
	};
	channel(0) | channel(8) | channel(16)
}
