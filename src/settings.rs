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
	init_opacity_range, is_checked, opacity, parse_text_or, set_checked,
	set_control_text, set_enabled, set_opacity,
};
use self::ids::*;
use crate::config::{
	Bars, Color, Crosshair, Dot, Outline, Shape, names_match_ignoring_case,
};
use crate::{overlay, profiles, render, win};

const COLOR_TEXT_LENGTH: usize = "#RRGGBB".len();
const LAST_SHELL_EXECUTE_ERROR_CODE: isize = 32;

const SHAPE_OPTIONS: [(Shape, PCWSTR); 2] =
	[(Shape::Rounded, w!("Rounded")), (Shape::Sharp, w!("Sharp"))];

thread_local! {
	static SETTINGS_DIALOG: Cell<HWND> = const { Cell::new(ptr::null_mut()) };
	static IS_WRITING_CONTROLS: Cell<bool> = const { Cell::new(false) };
}

pub fn dialog_window() -> HWND {
	SETTINGS_DIALOG.get()
}

pub fn open() {
	if SETTINGS_DIALOG.get().is_null() {
		let dialog = unsafe {
			CreateDialogParamW(
				win::module_instance(),
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
		SETTINGS_DIALOG.set(dialog);
	}
	unsafe {
		ShowWindow(SETTINGS_DIALOG.get(), SW_SHOW);
		SetForegroundWindow(SETTINGS_DIALOG.get());
	}
}

pub fn close() {
	let dialog = SETTINGS_DIALOG.get();
	if !dialog.is_null() {
		unsafe {
			DestroyWindow(dialog);
		}
	}
}

unsafe extern "system" fn dialog_proc(
	dialog: HWND,
	message: u32,
	wparam: WPARAM,
	lparam: LPARAM,
) -> isize {
	match message {
		WM_INITDIALOG => init_dialog_controls(dialog),
		WM_HSCROLL | WM_COMMAND
			if SETTINGS_DIALOG.get().is_null() || IS_WRITING_CONTROLS.get() => {},
		WM_HSCROLL => apply_edited_controls(dialog),
		WM_COMMAND => on_dialog_command(
			dialog,
			(wparam & 0xFFFF) as i32,
			(wparam >> 16 & 0xFFFF) as u32,
		),
		WM_SYSCOMMAND if wparam as u32 & 0xFFF0 == SC_CONTEXTHELP => {
			open_repository(dialog)
		},
		WM_DRAWITEM => {
			preview::draw_preview(unsafe { &*(lparam as *const DRAWITEMSTRUCT) })
		},
		WM_DESTROY => SETTINGS_DIALOG.set(ptr::null_mut()),
		_ => return 0,
	}
	1
}

fn init_dialog_controls(dialog: HWND) {
	unsafe {
		let large_icon = win::load_app_icon(GetSystemMetrics(SM_CXICON));
		let small_icon = win::load_app_icon(GetSystemMetrics(SM_CXSMICON));
		SendMessageW(dialog, WM_SETICON, ICON_BIG as usize, large_icon as isize);
		SendMessageW(dialog, WM_SETICON, ICON_SMALL as usize, small_icon as isize);
		for combo_box_id in SHAPE_COMBO_BOXES {
			for (_, shape_name) in SHAPE_OPTIONS {
				SendDlgItemMessageW(
					dialog,
					combo_box_id,
					CB_ADDSTRING,
					0,
					shape_name as isize,
				);
			}
		}
		for (number_edit_id, allowed_range) in NUMBER_EDITS_WITH_RANGES {
			let (min, max) = (*allowed_range.start(), *allowed_range.end());
			SendDlgItemMessageW(
				dialog,
				companion_control(number_edit_id),
				UDM_SETRANGE32,
				min as usize,
				max as isize,
			);
		}
		for color_edit_id in COLOR_EDITS {
			SendDlgItemMessageW(
				dialog,
				color_edit_id,
				EM_SETLIMITTEXT,
				COLOR_TEXT_LENGTH,
				0,
			);
		}
	}
	for trackbar_id in OPACITY_TRACKBARS {
		init_opacity_range(dialog, trackbar_id);
	}
	fill_profile_list(dialog);
	set_checked(dialog, ID_WINDOWS_SCALING, profiles::uses_windows_scaling());
	write_crosshair_to_controls(dialog, overlay::displayed_crosshair());
}

fn on_dialog_command(dialog: HWND, control_id: i32, notification_code: u32) {
	match (control_id, notification_code) {
		(IDCANCEL, _) => unsafe {
			DestroyWindow(dialog);
		},
		(ID_WINDOWS_SCALING, BN_CLICKED) => toggle_windows_scaling(dialog),
		(ID_PROFILE_LIST, CBN_SELCHANGE) => switch_to_selected_profile(dialog),
		(ID_NEW_PROFILE, BN_CLICKED) => {
			prompt_for_profile_name(dialog, String::new(), |requested_name| {
				profiles::create_profile(requested_name, overlay::displayed_crosshair())
			})
		},
		(ID_RENAME_PROFILE, BN_CLICKED) => prompt_for_profile_name(
			dialog,
			profiles::active_profile_name(),
			profiles::rename_active_profile,
		),
		(ID_DELETE_OR_RESET_PROFILE, BN_CLICKED)
			if profiles::is_default_profile_active() =>
		{
			load_crosshair_into_editor(dialog, Crosshair::default())
		},
		(ID_DELETE_OR_RESET_PROFILE, BN_CLICKED) => {
			confirm_and_delete_active_profile(dialog)
		},
		(ID_SAVE_PROFILE, BN_CLICKED) => save_active_profile(dialog),
		(ID_REVERT_CHANGES, BN_CLICKED) => {
			load_crosshair_into_editor(dialog, profiles::saved_crosshair())
		},
		_ => on_crosshair_control_command(dialog, control_id, notification_code),
	}
}

fn on_crosshair_control_command(dialog: HWND, control_id: i32, notification_code: u32) {
	let color_edit_for_picker_button = COLOR_EDITS
		.into_iter()
		.find(|&color_edit_id| companion_control(color_edit_id) == control_id);
	match (control_id, notification_code, color_edit_for_picker_button) {
		(ID_SHOW_BARS, BN_CLICKED, _) => {
			let show_all_sides = is_checked(dialog, ID_SHOW_BARS);
			for side_checkbox_id in ID_BAR_TOP..=ID_BAR_RIGHT {
				set_checked(dialog, side_checkbox_id, show_all_sides);
			}
			apply_edited_controls(dialog);
		},
		(_, BN_CLICKED, Some(color_edit_id)) => {
			pick_color_with_dialog(dialog, color_edit_id)
		},
		(_, EN_KILLFOCUS, _) => {
			write_crosshair_to_controls(dialog, overlay::displayed_crosshair())
		},
		(_, BN_CLICKED | EN_CHANGE | CBN_SELCHANGE, _) => apply_edited_controls(dialog),
		_ => {},
	}
}

fn open_repository(dialog: HWND) {
	let repository_url = win::null_terminated_utf16(env!("CARGO_PKG_REPOSITORY"));
	let shell_result = unsafe {
		ShellExecuteW(
			dialog,
			w!("open"),
			repository_url.as_ptr(),
			ptr::null(),
			ptr::null(),
			SW_SHOWNORMAL,
		)
	};
	if shell_result as isize <= LAST_SHELL_EXECUTE_ERROR_CODE {
		win::show_error(dialog, "Failed to open the repository.");
	}
}

fn toggle_windows_scaling(dialog: HWND) {
	let is_enabled = is_checked(dialog, ID_WINDOWS_SCALING);
	match profiles::set_uses_windows_scaling(is_enabled) {
		Ok(()) => {
			overlay::redraw();
			refresh_dependent_controls(dialog, overlay::displayed_crosshair());
		},
		Err(error) => {
			win::show_error(dialog, &format!("Failed to save the setting.\n\n{error}"));
			set_checked(dialog, ID_WINDOWS_SCALING, !is_enabled);
		},
	}
}

fn fill_profile_list(dialog: HWND) {
	let profile_names = profiles::profile_names();
	let active_name = profiles::active_profile_name();
	let selected_index = profile_names
		.iter()
		.position(|profile_name| names_match_ignoring_case(profile_name, &active_name))
		.unwrap_or(0);
	unsafe {
		SendDlgItemMessageW(dialog, ID_PROFILE_LIST, CB_RESETCONTENT, 0, 0);
		for profile_name in &profile_names {
			let wide_profile_name = win::null_terminated_utf16(profile_name);
			SendDlgItemMessageW(
				dialog,
				ID_PROFILE_LIST,
				CB_ADDSTRING,
				0,
				wide_profile_name.as_ptr() as isize,
			);
		}
		SendDlgItemMessageW(dialog, ID_PROFILE_LIST, CB_SETCURSEL, selected_index, 0);
	}
}

fn switch_to_selected_profile(dialog: HWND) {
	let selected_index =
		unsafe { SendDlgItemMessageW(dialog, ID_PROFILE_LIST, CB_GETCURSEL, 0, 0) };
	let profile_names = profiles::profile_names();
	let Some(selected_name) =
		usize::try_from(selected_index).ok().and_then(|index| profile_names.get(index))
	else {
		return;
	};
	let active_name = profiles::active_profile_name();
	let discard_question =
		format!("Discard your unsaved changes to \"{active_name}\"?");
	if names_match_ignoring_case(selected_name, &active_name)
		|| (has_unsaved_changes() && !win::ask_yes_no(dialog, &discard_question))
	{
		fill_profile_list(dialog);
		return;
	}
	match profiles::switch_to_profile(selected_name) {
		Ok(crosshair) => load_crosshair_into_editor(dialog, crosshair),
		Err(error) => {
			win::show_error(dialog, &format!("Failed to switch profiles.\n\n{error}"));
			fill_profile_list(dialog);
		},
	}
}

fn prompt_for_profile_name(
	dialog: HWND,
	initial_name: String,
	apply_name: impl Fn(&str) -> Result<(), String>,
) {
	let mut name_to_show = initial_name;
	while let Some(entered_name) = prompt::ask_profile_name(dialog, &name_to_show) {
		match apply_name(&entered_name) {
			Ok(()) => {
				fill_profile_list(dialog);
				refresh_dependent_controls(dialog, overlay::displayed_crosshair());
				return;
			},
			Err(error) => {
				win::show_error(dialog, &error);
				name_to_show = entered_name;
			},
		}
	}
}

fn confirm_and_delete_active_profile(dialog: HWND) {
	let confirmation_question =
		format!("Delete the \"{}\" profile?", profiles::active_profile_name());
	if !win::ask_yes_no(dialog, &confirmation_question) {
		return;
	}
	match profiles::delete_active_profile() {
		Ok(crosshair) => {
			fill_profile_list(dialog);
			load_crosshair_into_editor(dialog, crosshair);
		},
		Err(error) => win::show_error(
			dialog,
			&format!("Failed to delete the profile.\n\n{error}"),
		),
	}
}

fn save_active_profile(dialog: HWND) {
	if let Err(error) = profiles::save_active_profile(overlay::displayed_crosshair()) {
		win::show_error(dialog, &format!("Failed to save the profile.\n\n{error}"));
	}
	refresh_dependent_controls(dialog, overlay::displayed_crosshair());
}

fn has_unsaved_changes() -> bool {
	overlay::displayed_crosshair() != profiles::saved_crosshair()
}

fn load_crosshair_into_editor(dialog: HWND, crosshair: Crosshair) {
	overlay::display(crosshair);
	write_crosshair_to_controls(dialog, crosshair);
}

fn apply_edited_controls(dialog: HWND) {
	let crosshair =
		read_crosshair_from_controls(dialog, overlay::displayed_crosshair());
	overlay::display(crosshair);
	refresh_dependent_controls(dialog, crosshair);
}

fn read_crosshair_from_controls(
	dialog: HWND,
	fallback_crosshair: Crosshair,
) -> Crosshair {
	let Crosshair { bars, dot } = fallback_crosshair;
	Crosshair {
		bars: Bars {
			top: is_checked(dialog, ID_BAR_TOP),
			bottom: is_checked(dialog, ID_BAR_BOTTOM),
			left: is_checked(dialog, ID_BAR_LEFT),
			right: is_checked(dialog, ID_BAR_RIGHT),
			color: parse_text_or(dialog, ID_BAR_COLOR, bars.color),
			length: parse_text_or(dialog, ID_BAR_LENGTH, bars.length),
			thickness: parse_text_or(dialog, ID_BAR_THICKNESS, bars.thickness),
			gap: parse_text_or(dialog, ID_BAR_GAP, bars.gap),
			shape: read_shape(dialog, ID_BAR_SHAPE, bars.shape),
			opacity: opacity(dialog, ID_BAR_OPACITY),
			outline: read_outline(dialog, ID_BAR_OUTLINE, bars.outline),
		},
		dot: Dot {
			enabled: is_checked(dialog, ID_SHOW_DOT),
			color: parse_text_or(dialog, ID_DOT_COLOR, dot.color),
			size: parse_text_or(dialog, ID_DOT_SIZE, dot.size),
			shape: read_shape(dialog, ID_DOT_SHAPE, dot.shape),
			opacity: opacity(dialog, ID_DOT_OPACITY),
			outline: read_outline(dialog, ID_DOT_OUTLINE, dot.outline),
		},
	}
	.clamped_to_allowed_ranges()
}

fn read_shape(dialog: HWND, combo_box_id: i32, fallback_shape: Shape) -> Shape {
	let selected_index =
		unsafe { SendDlgItemMessageW(dialog, combo_box_id, CB_GETCURSEL, 0, 0) };
	usize::try_from(selected_index)
		.ok()
		.and_then(|index| SHAPE_OPTIONS.get(index))
		.map_or(fallback_shape, |&(shape, _)| shape)
}

fn read_outline(
	dialog: HWND,
	outline_checkbox_id: i32,
	fallback_outline: Outline,
) -> Outline {
	Outline {
		enabled: is_checked(dialog, outline_checkbox_id),
		color: parse_text_or(
			dialog,
			outline_checkbox_id + OUTLINE_COLOR_OFFSET,
			fallback_outline.color,
		),
		thickness: parse_text_or(
			dialog,
			outline_checkbox_id + OUTLINE_THICKNESS_OFFSET,
			fallback_outline.thickness,
		),
		opacity: opacity(dialog, outline_checkbox_id + OUTLINE_OPACITY_OFFSET),
	}
}

fn write_crosshair_to_controls(dialog: HWND, crosshair: Crosshair) {
	let Crosshair { bars, dot } = crosshair;
	IS_WRITING_CONTROLS.set(true);
	set_checked(dialog, ID_BAR_TOP, bars.top);
	set_checked(dialog, ID_BAR_BOTTOM, bars.bottom);
	set_checked(dialog, ID_BAR_LEFT, bars.left);
	set_checked(dialog, ID_BAR_RIGHT, bars.right);
	set_control_text(dialog, ID_BAR_COLOR, &bars.color.to_string());
	set_control_text(dialog, ID_BAR_LENGTH, &bars.length.to_string());
	set_control_text(dialog, ID_BAR_THICKNESS, &bars.thickness.to_string());
	set_control_text(dialog, ID_BAR_GAP, &bars.gap.to_string());
	write_shape(dialog, ID_BAR_SHAPE, bars.shape);
	set_opacity(dialog, ID_BAR_OPACITY, bars.opacity);
	write_outline(dialog, ID_BAR_OUTLINE, bars.outline);
	set_checked(dialog, ID_SHOW_DOT, dot.enabled);
	set_control_text(dialog, ID_DOT_COLOR, &dot.color.to_string());
	set_control_text(dialog, ID_DOT_SIZE, &dot.size.to_string());
	write_shape(dialog, ID_DOT_SHAPE, dot.shape);
	set_opacity(dialog, ID_DOT_OPACITY, dot.opacity);
	write_outline(dialog, ID_DOT_OUTLINE, dot.outline);
	IS_WRITING_CONTROLS.set(false);
	refresh_dependent_controls(dialog, crosshair);
}

fn write_shape(dialog: HWND, combo_box_id: i32, shape: Shape) {
	let shape_index =
		SHAPE_OPTIONS.iter().position(|&(listed_shape, _)| listed_shape == shape);
	unsafe {
		SendDlgItemMessageW(
			dialog,
			combo_box_id,
			CB_SETCURSEL,
			shape_index.expect("every shape is listed"),
			0,
		);
	}
}

fn write_outline(dialog: HWND, outline_checkbox_id: i32, outline: Outline) {
	set_checked(dialog, outline_checkbox_id, outline.enabled);
	set_control_text(
		dialog,
		outline_checkbox_id + OUTLINE_COLOR_OFFSET,
		&outline.color.to_string(),
	);
	set_control_text(
		dialog,
		outline_checkbox_id + OUTLINE_THICKNESS_OFFSET,
		&outline.thickness.to_string(),
	);
	set_opacity(dialog, outline_checkbox_id + OUTLINE_OPACITY_OFFSET, outline.opacity);
}

fn refresh_dependent_controls(dialog: HWND, crosshair: Crosshair) {
	let Crosshair { bars, dot } = crosshair;
	set_checked(dialog, ID_SHOW_BARS, bars.has_any_side_enabled());
	let opacity_values =
		[bars.opacity, bars.outline.opacity, dot.opacity, dot.outline.opacity];
	for (trackbar_id, opacity_value) in
		OPACITY_TRACKBARS.into_iter().zip(opacity_values)
	{
		set_control_text(
			dialog,
			companion_control(trackbar_id),
			&format!("{opacity_value:.2}"),
		);
	}
	set_enabled(dialog, ID_BAR_COLOR..=ID_BAR_OUTLINE, bars.has_any_side_enabled());
	set_enabled(
		dialog,
		outline_detail_controls(ID_BAR_OUTLINE),
		bars.has_any_side_enabled() && bars.outline.enabled,
	);
	set_enabled(dialog, ID_DOT_COLOR..=ID_DOT_OUTLINE, dot.enabled);
	set_enabled(
		dialog,
		outline_detail_controls(ID_DOT_OUTLINE),
		dot.enabled && dot.outline.enabled,
	);
	let is_default_profile = profiles::is_default_profile_active();
	set_control_text(
		dialog,
		ID_DELETE_OR_RESET_PROFILE,
		if is_default_profile { "Reset" } else { "Delete" },
	);
	set_enabled(dialog, [ID_RENAME_PROFILE], !is_default_profile);
	set_enabled(
		dialog,
		[ID_DELETE_OR_RESET_PROFILE],
		!is_default_profile || crosshair != Crosshair::default(),
	);
	set_enabled(
		dialog,
		ID_SAVE_PROFILE..=ID_REVERT_CHANGES,
		crosshair != profiles::saved_crosshair(),
	);
	let scale_factor = overlay::scale_factor();
	let (width, height) =
		render::render_crosshair(&crosshair, scale_factor).visible_bounds_size();
	let scale_percent = (scale_factor * 100.0).round();
	set_control_text(
		dialog,
		ID_VISIBLE_SIZE_LABEL,
		&format!("{width}x{height} ({scale_percent}% scaling)"),
	);
	unsafe {
		InvalidateRect(GetDlgItem(dialog, ID_PREVIEW), ptr::null(), 0);
	}
}

fn outline_detail_controls(outline_checkbox_id: i32) -> RangeInclusive<i32> {
	outline_checkbox_id + OUTLINE_COLOR_OFFSET
		..=companion_control(outline_checkbox_id + OUTLINE_OPACITY_OFFSET)
}

fn pick_color_with_dialog(dialog: HWND, color_edit_id: i32) {
	let Color { r: red, g: green, b: blue } =
		parse_text_or(dialog, color_edit_id, Color::default());
	let mut custom_colors = [0x00FFFFFF; 16];
	let mut color_dialog_options = CHOOSECOLORW {
		lStructSize: mem::size_of::<CHOOSECOLORW>() as u32,
		hwndOwner: dialog,
		rgbResult: u32::from_le_bytes([red, green, blue, 0]),
		lpCustColors: custom_colors.as_mut_ptr(),
		Flags: CC_RGBINIT | CC_FULLOPEN,
		..unsafe { mem::zeroed() }
	};
	if unsafe { ChooseColorW(&mut color_dialog_options) } != 0 {
		let [red, green, blue, _] = color_dialog_options.rgbResult.to_le_bytes();
		set_control_text(
			dialog,
			color_edit_id,
			&Color { r: red, g: green, b: blue }.to_string(),
		);
	}
}
