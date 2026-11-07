use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::UI::Controls::DRAWITEMSTRUCT;

use crate::{overlay, render, win};

const PREVIEW_BACKGROUND_COLOR: u32 = 0x00404040;
const MAX_ZOOM: i32 = 16;
const IMAGE_MARGIN_PIXELS: i32 = 2;

pub fn draw_preview(draw_item: &DRAWITEMSTRUCT) {
	let crosshair_image = render::render_crosshair(
		&overlay::displayed_crosshair(),
		overlay::scale_factor(),
	);
	let preview_rect = draw_item.rcItem;
	let (width, height) = (
		preview_rect.right - preview_rect.left,
		preview_rect.bottom - preview_rect.top,
	);
	let square_side = width.min(height);
	let zoom = (square_side / (crosshair_image.side_length + 2 * IMAGE_MARGIN_PIXELS))
		.clamp(1, MAX_ZOOM);
	let image_left = (width - crosshair_image.side_length * zoom) / 2;
	let image_top = (height - crosshair_image.side_length * zoom) / 2;
	let image_pixel_range = 0..crosshair_image.side_length;
	let square_columns = (width - square_side) / 2..(width + square_side) / 2;
	let square_rows = (height - square_side) / 2..(height + square_side) / 2;
	let [red, green, blue, _] = unsafe { GetSysColor(COLOR_BTNFACE) }.to_le_bytes();
	let button_face_color = u32::from_be_bytes([0, red, green, blue]);

	let mut frame_pixels = Vec::with_capacity((width * height) as usize);
	for frame_y in 0..height {
		let image_row = (frame_y - image_top).div_euclid(zoom);
		for frame_x in 0..width {
			let image_column = (frame_x - image_left).div_euclid(zoom);
			let frame_pixel = if !square_columns.contains(&frame_x)
				|| !square_rows.contains(&frame_y)
			{
				button_face_color
			} else if image_pixel_range.contains(&image_row)
				&& image_pixel_range.contains(&image_column)
			{
				composite_over_preview_background(
					crosshair_image.pixels[(image_row * crosshair_image.side_length
						+ image_column) as usize],
				)
			} else {
				PREVIEW_BACKGROUND_COLOR
			};
			frame_pixels.push(frame_pixel);
		}
	}
	let bitmap_info = win::top_down_32_bit_bitmap_info(width, height);
	unsafe {
		SetDIBitsToDevice(
			draw_item.hDC,
			preview_rect.left,
			preview_rect.top,
			width as u32,
			height as u32,
			0,
			0,
			0,
			height as u32,
			frame_pixels.as_ptr().cast(),
			&bitmap_info,
			DIB_RGB_COLORS,
		);
	}
}

fn composite_over_preview_background(premultiplied_pixel: u32) -> u32 {
	let top_transparency = 255 - (premultiplied_pixel >> 24);
	let composited_channel = |bit_shift: u32| {
		let channel_on_top = (premultiplied_pixel >> bit_shift) & 0xFF;
		let channel_below = (PREVIEW_BACKGROUND_COLOR >> bit_shift) & 0xFF;
		(channel_on_top + channel_below * top_transparency / 255) << bit_shift
	};
	composited_channel(0) | composited_channel(8) | composited_channel(16)
}
