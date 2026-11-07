mod controls;
mod ids;
mod preview;
mod prompt;

use std::cell::Cell;
use std::ops::RangeInclusive;
use std::{mem, ptr};

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::UI::Controls::Dialogs::*;
use windows_sys::Win32::UI::Controls::*;
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use windows_sys::core::PCWSTR;
use windows_sys::w;

use self::controls::{
	checked, enable, init_opacity, opacity, parse, set_checked, set_opacity, set_text,
};
use self::ids::*;
use crate::config::{Bars, Color, Crosshair, Dot, Outline, Shape, same_name};
use crate::{overlay, profiles, render, win};

const SHAPE_NAMES: [(Shape, PCWSTR); 2] =
	[(Shape::Rounded, w!("Rounded")), (Shape::Sharp, w!("Sharp"))];

thread_local! {
	static DIALOG: Cell<HWND> = const { Cell::new(ptr::null_mut()) };
	static UPDATING: Cell<bool> = const { Cell::new(false) };
}

pub fn dialog() -> HWND {
	DIALOG.get()
}

pub fn open() {
	if DIALOG.get().is_null() {
		let dialog = unsafe {
			CreateDialogParamW(
				win::instance(),
				IDD_SETTINGS as PCWSTR,
				ptr::null_mut(),
				Some(dialog_proc),
				0,
			)
		};
		if dialog.is_null() {
			win::show_error(ptr::null_mut(), "Failed to open settings.");
			return;
		}
		DIALOG.set(dialog);
	}
	unsafe {
		ShowWindow(DIALOG.get(), SW_SHOW);
		SetForegroundWindow(DIALOG.get());
	}
}

pub fn close() {
	let dialog = DIALOG.get();
	if !dialog.is_null() {
		unsafe {
			DestroyWindow(dialog);
		}
	}
}

unsafe extern "system" fn dialog_proc(
	dialog: HWND,
	msg: u32,
	wparam: WPARAM,
	lparam: LPARAM,
) -> isize {
	match msg {
		WM_INITDIALOG => init(dialog),
		WM_HSCROLL | WM_COMMAND if DIALOG.get().is_null() || UPDATING.get() => {},
		WM_HSCROLL => changed(dialog),
		WM_COMMAND => {
			on_command(dialog, (wparam & 0xFFFF) as i32, (wparam >> 16 & 0xFFFF) as u32)
		},
		WM_SYSCOMMAND if wparam as u32 & 0xFFF0 == SC_CONTEXTHELP => {
			open_repository(dialog)
		},
		WM_DRAWITEM => preview::draw(unsafe { &*(lparam as *const DRAWITEMSTRUCT) }),
		WM_DESTROY => DIALOG.set(ptr::null_mut()),
		_ => return 0,
	}
	1
}

fn init(dialog: HWND) {
	unsafe {
		let big = win::icon(GetSystemMetrics(SM_CXICON));
		let small = win::icon(GetSystemMetrics(SM_CXSMICON));
		SendMessageW(dialog, WM_SETICON, ICON_BIG as usize, big as isize);
		SendMessageW(dialog, WM_SETICON, ICON_SMALL as usize, small as isize);
		for id in SHAPES {
			for (_, name) in SHAPE_NAMES {
				SendDlgItemMessageW(dialog, id, CB_ADDSTRING, 0, name as isize);
			}
		}
		for (id, range) in NUMBERS {
			let (min, max) = (*range.start(), *range.end());
			SendDlgItemMessageW(
				dialog,
				companion(id),
				UDM_SETRANGE32,
				min as usize,
				max as isize,
			);
		}
		for id in COLORS {
			SendDlgItemMessageW(dialog, id, EM_SETLIMITTEXT, 7, 0);
		}
	}
	for id in OPACITIES {
		init_opacity(dialog, id);
	}
	fill_profiles(dialog);
	set_checked(dialog, ID_SCALING, profiles::windows_scaling());
	write(dialog, overlay::crosshair());
}

fn on_command(dialog: HWND, id: i32, code: u32) {
	match (id, code) {
		(IDCANCEL, _) => unsafe {
			DestroyWindow(dialog);
		},
		(ID_SCALING, BN_CLICKED) => toggle_scaling(dialog),
		(ID_PROFILE, CBN_SELCHANGE) => switch_profile(dialog),
		(ID_NEW, BN_CLICKED) => name_profile(dialog, String::new(), |name| {
			profiles::create(name, overlay::crosshair())
		}),
		(ID_RENAME, BN_CLICKED) => {
			name_profile(dialog, profiles::active(), profiles::rename)
		},
		(ID_DELETE, BN_CLICKED) if profiles::is_default() => {
			show(dialog, Crosshair::default())
		},
		(ID_DELETE, BN_CLICKED) => delete_profile(dialog),
		(ID_SAVE, BN_CLICKED) => save_profile(dialog),
		(ID_REVERT, BN_CLICKED) => show(dialog, profiles::saved()),
		_ => on_setting_command(dialog, id, code),
	}
}

fn on_setting_command(dialog: HWND, id: i32, code: u32) {
	let picked_color = COLORS.into_iter().find(|&color| companion(color) == id);
	match (id, code, picked_color) {
		(ID_BARS, BN_CLICKED, _) => {
			let on = checked(dialog, ID_BARS);
			for side in ID_BAR_TOP..=ID_BAR_RIGHT {
				set_checked(dialog, side, on);
			}
			changed(dialog);
		},
		(_, BN_CLICKED, Some(color)) => pick_color(dialog, color),
		(_, EN_KILLFOCUS, _) => write(dialog, overlay::crosshair()),
		(_, BN_CLICKED | EN_CHANGE | CBN_SELCHANGE, _) => changed(dialog),
		_ => {},
	}
}

fn open_repository(dialog: HWND) {
	let url = win::wide(env!("CARGO_PKG_REPOSITORY"));
	let result = unsafe {
		ShellExecuteW(
			dialog,
			w!("open"),
			url.as_ptr(),
			ptr::null(),
			ptr::null(),
			SW_SHOWNORMAL,
		)
	};
	if result as isize <= 32 {
		win::show_error(dialog, "Failed to open the repository.");
	}
}

fn toggle_scaling(dialog: HWND) {
	let on = checked(dialog, ID_SCALING);
	match profiles::set_windows_scaling(on) {
		Ok(()) => {
			overlay::refresh();
			sync(dialog, overlay::crosshair());
		},
		Err(e) => {
			win::show_error(dialog, &format!("Failed to save the setting.\n\n{e}"));
			set_checked(dialog, ID_SCALING, !on);
		},
	}
}

fn fill_profiles(dialog: HWND) {
	let names = profiles::names();
	let active = profiles::active();
	let selected = names.iter().position(|name| same_name(name, &active)).unwrap_or(0);
	unsafe {
		SendDlgItemMessageW(dialog, ID_PROFILE, CB_RESETCONTENT, 0, 0);
		for name in &names {
			let name = win::wide(name);
			SendDlgItemMessageW(
				dialog,
				ID_PROFILE,
				CB_ADDSTRING,
				0,
				name.as_ptr() as isize,
			);
		}
		SendDlgItemMessageW(dialog, ID_PROFILE, CB_SETCURSEL, selected, 0);
	}
}

fn switch_profile(dialog: HWND) {
	let index = unsafe { SendDlgItemMessageW(dialog, ID_PROFILE, CB_GETCURSEL, 0, 0) };
	let names = profiles::names();
	let Some(name) = usize::try_from(index).ok().and_then(|index| names.get(index))
	else {
		return;
	};
	let active = profiles::active();
	let discard = format!("Discard your unsaved changes to \"{active}\"?");
	if same_name(name, &active) || (has_changes() && !win::confirm(dialog, &discard)) {
		fill_profiles(dialog);
		return;
	}
	match profiles::select(name) {
		Ok(crosshair) => show(dialog, crosshair),
		Err(e) => {
			win::show_error(dialog, &format!("Failed to switch profiles.\n\n{e}"));
			fill_profiles(dialog);
		},
	}
}

fn name_profile(
	dialog: HWND,
	initial: String,
	apply: impl Fn(&str) -> Result<(), String>,
) {
	let mut name = initial;
	while let Some(input) = prompt::ask_name(dialog, &name) {
		match apply(&input) {
			Ok(()) => {
				fill_profiles(dialog);
				sync(dialog, overlay::crosshair());
				return;
			},
			Err(e) => {
				win::show_error(dialog, &e);
				name = input;
			},
		}
	}
}

fn delete_profile(dialog: HWND) {
	let message = format!("Delete the \"{}\" profile?", profiles::active());
	if !win::confirm(dialog, &message) {
		return;
	}
	match profiles::delete() {
		Ok(crosshair) => {
			fill_profiles(dialog);
			show(dialog, crosshair);
		},
		Err(e) => {
			win::show_error(dialog, &format!("Failed to delete the profile.\n\n{e}"))
		},
	}
}

fn save_profile(dialog: HWND) {
	if let Err(e) = profiles::save(overlay::crosshair()) {
		win::show_error(dialog, &format!("Failed to save the profile.\n\n{e}"));
	}
	sync(dialog, overlay::crosshair());
}

fn has_changes() -> bool {
	overlay::crosshair() != profiles::saved()
}

fn show(dialog: HWND, crosshair: Crosshair) {
	overlay::apply(crosshair);
	write(dialog, crosshair);
}

fn changed(dialog: HWND) {
	let crosshair = read(dialog, overlay::crosshair());
	overlay::apply(crosshair);
	sync(dialog, crosshair);
}

fn read(dialog: HWND, previous: Crosshair) -> Crosshair {
	let Crosshair { bars, dot } = previous;
	Crosshair {
		bars: Bars {
			top: checked(dialog, ID_BAR_TOP),
			bottom: checked(dialog, ID_BAR_BOTTOM),
			left: checked(dialog, ID_BAR_LEFT),
			right: checked(dialog, ID_BAR_RIGHT),
			color: parse(dialog, ID_BAR_COLOR, bars.color),
			length: parse(dialog, ID_BAR_LENGTH, bars.length),
			thickness: parse(dialog, ID_BAR_THICKNESS, bars.thickness),
			gap: parse(dialog, ID_BAR_GAP, bars.gap),
			shape: read_shape(dialog, ID_BAR_SHAPE, bars.shape),
			opacity: opacity(dialog, ID_BAR_OPACITY),
			outline: read_outline(dialog, ID_BAR_OUTLINE, bars.outline),
		},
		dot: Dot {
			enabled: checked(dialog, ID_DOT),
			color: parse(dialog, ID_DOT_COLOR, dot.color),
			size: parse(dialog, ID_DOT_SIZE, dot.size),
			shape: read_shape(dialog, ID_DOT_SHAPE, dot.shape),
			opacity: opacity(dialog, ID_DOT_OPACITY),
			outline: read_outline(dialog, ID_DOT_OUTLINE, dot.outline),
		},
	}
	.clamped()
}

fn read_shape(dialog: HWND, id: i32, previous: Shape) -> Shape {
	let index = unsafe { SendDlgItemMessageW(dialog, id, CB_GETCURSEL, 0, 0) };
	usize::try_from(index)
		.ok()
		.and_then(|index| SHAPE_NAMES.get(index))
		.map_or(previous, |&(shape, _)| shape)
}

fn read_outline(dialog: HWND, id: i32, previous: Outline) -> Outline {
	Outline {
		enabled: checked(dialog, id),
		color: parse(dialog, id + OUTLINE_COLOR, previous.color),
		thickness: parse(dialog, id + OUTLINE_THICKNESS, previous.thickness),
		opacity: opacity(dialog, id + OUTLINE_OPACITY),
	}
}

fn write(dialog: HWND, crosshair: Crosshair) {
	let Crosshair { bars, dot } = crosshair;
	UPDATING.set(true);
	set_checked(dialog, ID_BAR_TOP, bars.top);
	set_checked(dialog, ID_BAR_BOTTOM, bars.bottom);
	set_checked(dialog, ID_BAR_LEFT, bars.left);
	set_checked(dialog, ID_BAR_RIGHT, bars.right);
	set_text(dialog, ID_BAR_COLOR, &bars.color.to_string());
	set_text(dialog, ID_BAR_LENGTH, &bars.length.to_string());
	set_text(dialog, ID_BAR_THICKNESS, &bars.thickness.to_string());
	set_text(dialog, ID_BAR_GAP, &bars.gap.to_string());
	write_shape(dialog, ID_BAR_SHAPE, bars.shape);
	set_opacity(dialog, ID_BAR_OPACITY, bars.opacity);
	write_outline(dialog, ID_BAR_OUTLINE, bars.outline);
	set_checked(dialog, ID_DOT, dot.enabled);
	set_text(dialog, ID_DOT_COLOR, &dot.color.to_string());
	set_text(dialog, ID_DOT_SIZE, &dot.size.to_string());
	write_shape(dialog, ID_DOT_SHAPE, dot.shape);
	set_opacity(dialog, ID_DOT_OPACITY, dot.opacity);
	write_outline(dialog, ID_DOT_OUTLINE, dot.outline);
	UPDATING.set(false);
	sync(dialog, crosshair);
}

fn write_shape(dialog: HWND, id: i32, shape: Shape) {
	let index = SHAPE_NAMES.iter().position(|&(option, _)| option == shape);
	unsafe {
		SendDlgItemMessageW(
			dialog,
			id,
			CB_SETCURSEL,
			index.expect("every shape is listed"),
			0,
		);
	}
}

fn write_outline(dialog: HWND, id: i32, outline: Outline) {
	set_checked(dialog, id, outline.enabled);
	set_text(dialog, id + OUTLINE_COLOR, &outline.color.to_string());
	set_text(dialog, id + OUTLINE_THICKNESS, &outline.thickness.to_string());
	set_opacity(dialog, id + OUTLINE_OPACITY, outline.opacity);
}

fn sync(dialog: HWND, crosshair: Crosshair) {
	let Crosshair { bars, dot } = crosshair;
	set_checked(dialog, ID_BARS, bars.any());
	let opacities =
		[bars.opacity, bars.outline.opacity, dot.opacity, dot.outline.opacity];
	for (id, value) in OPACITIES.into_iter().zip(opacities) {
		set_text(dialog, companion(id), &format!("{value:.2}"));
	}
	enable(dialog, ID_BAR_COLOR..=ID_BAR_OUTLINE, bars.any());
	enable(
		dialog,
		outline_controls(ID_BAR_OUTLINE),
		bars.any() && bars.outline.enabled,
	);
	enable(dialog, ID_DOT_COLOR..=ID_DOT_OUTLINE, dot.enabled);
	enable(
		dialog,
		outline_controls(ID_DOT_OUTLINE),
		dot.enabled && dot.outline.enabled,
	);
	let default = profiles::is_default();
	set_text(dialog, ID_DELETE, if default { "Reset" } else { "Delete" });
	enable(dialog, [ID_RENAME], !default);
	enable(dialog, [ID_DELETE], !default || crosshair != Crosshair::default());
	enable(dialog, ID_SAVE..=ID_REVERT, crosshair != profiles::saved());
	let scale = overlay::scale();
	let (width, height) = render::render(&crosshair, scale).visible_size();
	let percent = (scale * 100.0).round();
	set_text(dialog, ID_GRID, &format!("{width}x{height} ({percent}% scaling)"));
	unsafe {
		InvalidateRect(GetDlgItem(dialog, ID_PREVIEW), ptr::null(), 0);
	}
}

fn outline_controls(id: i32) -> RangeInclusive<i32> {
	id + OUTLINE_COLOR..=companion(id + OUTLINE_OPACITY)
}

fn pick_color(dialog: HWND, id: i32) {
	let Color { r, g, b } = parse(dialog, id, Color::default());
	let mut custom_colors = [0x00FFFFFF; 16];
	let mut choose = CHOOSECOLORW {
		lStructSize: mem::size_of::<CHOOSECOLORW>() as u32,
		hwndOwner: dialog,
		rgbResult: u32::from_le_bytes([r, g, b, 0]),
		lpCustColors: custom_colors.as_mut_ptr(),
		Flags: CC_RGBINIT | CC_FULLOPEN,
		..unsafe { mem::zeroed() }
	};
	if unsafe { ChooseColorW(&mut choose) } != 0 {
		let [r, g, b, _] = choose.rgbResult.to_le_bytes();
		set_text(dialog, id, &Color { r, g, b }.to_string());
	}
}
