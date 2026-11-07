use std::ops::RangeInclusive;

use crate::config::{
	BAR_GAP_RANGE, BAR_LENGTH_RANGE, BAR_THICKNESS_RANGE, DOT_SIZE_RANGE,
	OUTLINE_THICKNESS_RANGE,
};

pub const IDD_SETTINGS: usize = 100;
pub const IDD_PROFILE_NAME_PROMPT: usize = 200;
pub const ID_PROFILE_NAME_EDIT: i32 = 201;
pub const ID_PREVIEW: i32 = 101;
pub const ID_WINDOWS_SCALING: i32 = 102;
pub const ID_PROFILE_LIST: i32 = 103;
pub const ID_NEW_PROFILE: i32 = 104;
pub const ID_RENAME_PROFILE: i32 = 105;
pub const ID_DELETE_OR_RESET_PROFILE: i32 = 106;
pub const ID_SAVE_PROFILE: i32 = 107;
pub const ID_REVERT_CHANGES: i32 = 108;
pub const ID_VISIBLE_SIZE_LABEL: i32 = 109;
pub const ID_SHOW_BARS: i32 = 110;
pub const ID_BAR_TOP: i32 = 111;
pub const ID_BAR_BOTTOM: i32 = 112;
pub const ID_BAR_LEFT: i32 = 113;
pub const ID_BAR_RIGHT: i32 = 114;
pub const ID_BAR_COLOR: i32 = 115;
pub const ID_BAR_LENGTH: i32 = 117;
pub const ID_BAR_THICKNESS: i32 = 119;
pub const ID_BAR_GAP: i32 = 121;
pub const ID_BAR_SHAPE: i32 = 123;
pub const ID_BAR_OPACITY: i32 = 124;
pub const ID_BAR_OUTLINE: i32 = 126;
pub const ID_SHOW_DOT: i32 = 140;
pub const ID_DOT_COLOR: i32 = 141;
pub const ID_DOT_SIZE: i32 = 143;
pub const ID_DOT_SHAPE: i32 = 145;
pub const ID_DOT_OPACITY: i32 = 146;
pub const ID_DOT_OUTLINE: i32 = 148;

/// The control that follows a color edit, number edit or opacity trackbar in the
/// dialog template: its picker button, up-down arrows or value label.
pub const fn companion_control(control_id: i32) -> i32 {
	control_id + 1
}

pub const OUTLINE_COLOR_OFFSET: i32 = 1;
pub const OUTLINE_THICKNESS_OFFSET: i32 = 3;
pub const OUTLINE_OPACITY_OFFSET: i32 = 5;

pub const SHAPE_COMBO_BOXES: [i32; 2] = [ID_BAR_SHAPE, ID_DOT_SHAPE];
pub const COLOR_EDITS: [i32; 4] = [
	ID_BAR_COLOR,
	ID_BAR_OUTLINE + OUTLINE_COLOR_OFFSET,
	ID_DOT_COLOR,
	ID_DOT_OUTLINE + OUTLINE_COLOR_OFFSET,
];
pub const NUMBER_EDITS_WITH_RANGES: [(i32, RangeInclusive<i32>); 6] = [
	(ID_BAR_LENGTH, BAR_LENGTH_RANGE),
	(ID_BAR_THICKNESS, BAR_THICKNESS_RANGE),
	(ID_BAR_GAP, BAR_GAP_RANGE),
	(ID_BAR_OUTLINE + OUTLINE_THICKNESS_OFFSET, OUTLINE_THICKNESS_RANGE),
	(ID_DOT_SIZE, DOT_SIZE_RANGE),
	(ID_DOT_OUTLINE + OUTLINE_THICKNESS_OFFSET, OUTLINE_THICKNESS_RANGE),
];
pub const OPACITY_TRACKBARS: [i32; 4] = [
	ID_BAR_OPACITY,
	ID_BAR_OUTLINE + OUTLINE_OPACITY_OFFSET,
	ID_DOT_OPACITY,
	ID_DOT_OUTLINE + OUTLINE_OPACITY_OFFSET,
];
