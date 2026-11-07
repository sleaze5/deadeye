use std::ops::RangeInclusive;

use crate::config::{
	DOT_SIZE_RANGE, GAP_RANGE, LENGTH_RANGE, OUTLINE_RANGE, THICKNESS_RANGE,
};

pub const IDD_SETTINGS: usize = 100;
pub const IDD_NAME: usize = 200;
pub const ID_NAME: i32 = 201;
pub const ID_PREVIEW: i32 = 101;
pub const ID_SCALING: i32 = 102;
pub const ID_PROFILE: i32 = 103;
pub const ID_NEW: i32 = 104;
pub const ID_RENAME: i32 = 105;
pub const ID_DELETE: i32 = 106;
pub const ID_SAVE: i32 = 107;
pub const ID_REVERT: i32 = 108;
pub const ID_GRID: i32 = 109;
pub const ID_BARS: i32 = 110;
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
pub const ID_DOT: i32 = 140;
pub const ID_DOT_COLOR: i32 = 141;
pub const ID_DOT_SIZE: i32 = 143;
pub const ID_DOT_SHAPE: i32 = 145;
pub const ID_DOT_OPACITY: i32 = 146;
pub const ID_DOT_OUTLINE: i32 = 148;

/// The control that follows a color edit, number edit or opacity trackbar in the
/// dialog template: its picker button, up-down arrows or value label.
pub const fn companion(id: i32) -> i32 {
	id + 1
}

pub const OUTLINE_COLOR: i32 = 1;
pub const OUTLINE_THICKNESS: i32 = 3;
pub const OUTLINE_OPACITY: i32 = 5;

pub const SHAPES: [i32; 2] = [ID_BAR_SHAPE, ID_DOT_SHAPE];
pub const COLORS: [i32; 4] = [
	ID_BAR_COLOR,
	ID_BAR_OUTLINE + OUTLINE_COLOR,
	ID_DOT_COLOR,
	ID_DOT_OUTLINE + OUTLINE_COLOR,
];
pub const NUMBERS: [(i32, RangeInclusive<i32>); 6] = [
	(ID_BAR_LENGTH, LENGTH_RANGE),
	(ID_BAR_THICKNESS, THICKNESS_RANGE),
	(ID_BAR_GAP, GAP_RANGE),
	(ID_BAR_OUTLINE + OUTLINE_THICKNESS, OUTLINE_RANGE),
	(ID_DOT_SIZE, DOT_SIZE_RANGE),
	(ID_DOT_OUTLINE + OUTLINE_THICKNESS, OUTLINE_RANGE),
];
pub const OPACITIES: [i32; 4] = [
	ID_BAR_OPACITY,
	ID_BAR_OUTLINE + OUTLINE_OPACITY,
	ID_DOT_OPACITY,
	ID_DOT_OUTLINE + OUTLINE_OPACITY,
];
