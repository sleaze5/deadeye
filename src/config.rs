use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::{fmt, fs, io, iter};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_json::ser::{PrettyFormatter, Serializer};

pub const BAR_LENGTH_RANGE: RangeInclusive<i32> = 1..=50;
pub const BAR_THICKNESS_RANGE: RangeInclusive<i32> = 1..=20;
pub const BAR_GAP_RANGE: RangeInclusive<i32> = 0..=50;
pub const DOT_SIZE_RANGE: RangeInclusive<i32> = 1..=30;
pub const OUTLINE_THICKNESS_RANGE: RangeInclusive<i32> = 1..=10;

pub const DEFAULT_PROFILE_NAME: &str = "Default";

const FILE_FORMAT_VERSION: u32 = 1;

#[derive(Clone)]
pub struct Config {
	pub active_profile_name: String,
	pub uses_windows_scaling: bool,
	pub profiles: Vec<Profile>,
}

#[derive(Clone, Serialize)]
pub struct Profile {
	pub name: String,
	pub crosshair: Crosshair,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Crosshair {
	pub bars: Bars,
	pub dot: Dot,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bars {
	pub top: bool,
	pub bottom: bool,
	pub left: bool,
	pub right: bool,
	pub color: Color,
	pub length: i32,
	pub thickness: i32,
	pub gap: i32,
	pub shape: Shape,
	pub opacity: f32,
	pub outline: Outline,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Dot {
	pub enabled: bool,
	pub color: Color,
	pub size: i32,
	pub shape: Shape,
	pub opacity: f32,
	pub outline: Outline,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Outline {
	pub enabled: bool,
	pub color: Color,
	pub thickness: i32,
	pub opacity: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Shape {
	Rounded,
	Sharp,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Color {
	pub r: u8,
	pub g: u8,
	pub b: u8,
}

const WHITE: Color = Color { r: 0xFF, g: 0xFF, b: 0xFF };

impl Default for Bars {
	fn default() -> Self {
		Self {
			top: true,
			bottom: true,
			left: true,
			right: true,
			color: WHITE,
			length: 6,
			thickness: 1,
			gap: 12,
			shape: Shape::Rounded,
			opacity: 1.0,
			outline: Outline::default(),
		}
	}
}

impl Default for Dot {
	fn default() -> Self {
		Self {
			enabled: true,
			color: WHITE,
			size: 2,
			shape: Shape::Rounded,
			opacity: 1.0,
			outline: Outline::default(),
		}
	}
}

impl Default for Outline {
	fn default() -> Self {
		Self { enabled: true, color: Color::default(), thickness: 1, opacity: 1.0 }
	}
}

impl Default for Config {
	fn default() -> Self {
		config_from_json(&Value::Null)
	}
}

impl Serialize for Config {
	fn serialize<S: serde::Serializer>(
		&self,
		serializer: S,
	) -> Result<S::Ok, S::Error> {
		#[derive(Serialize)]
		struct File<'a> {
			version: u32,
			active: &'a str,
			windows_scaling: bool,
			default: Crosshair,
			profiles: &'a [Profile],
		}
		let (default, profiles) =
			self.profiles.split_first().expect("the default profile exists");
		File {
			version: FILE_FORMAT_VERSION,
			active: &self.active_profile_name,
			windows_scaling: self.uses_windows_scaling,
			default: default.crosshair,
			profiles,
		}
		.serialize(serializer)
	}
}

impl Config {
	pub fn profile_named(&self, name: &str) -> Option<&Profile> {
		self.profiles
			.iter()
			.find(|profile| names_match_ignoring_case(&profile.name, name))
	}

	pub fn active_profile(&self) -> &Profile {
		self.profile_named(&self.active_profile_name)
			.expect("the active profile exists")
	}

	pub fn active_profile_mut(&mut self) -> &mut Profile {
		self.profiles
			.iter_mut()
			.find(|profile| {
				names_match_ignoring_case(&profile.name, &self.active_profile_name)
			})
			.expect("the active profile exists")
	}

	pub fn is_default_profile_active(&self) -> bool {
		names_match_ignoring_case(&self.active_profile_name, DEFAULT_PROFILE_NAME)
	}

	fn normalized(mut self) -> Self {
		let mut seen_names: Vec<String> = Vec::new();
		self.profiles.retain(|profile| {
			let is_first_with_this_name = !profile.name.trim().is_empty()
				&& !seen_names
					.iter()
					.any(|name| names_match_ignoring_case(name, &profile.name));
			seen_names.push(profile.name.clone());
			is_first_with_this_name
		});
		for profile in &mut self.profiles {
			profile.crosshair = profile.crosshair.clamped_to_allowed_ranges();
		}
		self.active_profile_name = self
			.profile_named(&self.active_profile_name)
			.map_or(DEFAULT_PROFILE_NAME.into(), |profile| profile.name.clone());
		self
	}
}

pub fn names_match_ignoring_case(first_name: &str, second_name: &str) -> bool {
	first_name.to_lowercase() == second_name.to_lowercase()
}

impl Crosshair {
	pub fn clamped_to_allowed_ranges(self) -> Self {
		Self {
			bars: Bars {
				length: clamp_to_range(self.bars.length, &BAR_LENGTH_RANGE),
				thickness: clamp_to_range(self.bars.thickness, &BAR_THICKNESS_RANGE),
				gap: clamp_to_range(self.bars.gap, &BAR_GAP_RANGE),
				opacity: self.bars.opacity.clamp(0.0, 1.0),
				outline: self.bars.outline.clamped_to_allowed_ranges(),
				..self.bars
			},
			dot: Dot {
				size: clamp_to_range(self.dot.size, &DOT_SIZE_RANGE),
				opacity: self.dot.opacity.clamp(0.0, 1.0),
				outline: self.dot.outline.clamped_to_allowed_ranges(),
				..self.dot
			},
		}
	}
}

impl Bars {
	pub fn has_any_side_enabled(&self) -> bool {
		self.top || self.bottom || self.left || self.right
	}
}

impl Outline {
	fn clamped_to_allowed_ranges(self) -> Self {
		Self {
			thickness: clamp_to_range(self.thickness, &OUTLINE_THICKNESS_RANGE),
			opacity: self.opacity.clamp(0.0, 1.0),
			..self
		}
	}
}

fn clamp_to_range(value: i32, range: &RangeInclusive<i32>) -> i32 {
	value.clamp(*range.start(), *range.end())
}

impl FromStr for Color {
	type Err = String;

	fn from_str(text: &str) -> Result<Self, String> {
		let hex_digits = text.strip_prefix('#').unwrap_or(text);
		if hex_digits.len() != 6
			|| !hex_digits.bytes().all(|byte| byte.is_ascii_hexdigit())
		{
			return Err(format!("invalid color \"{text}\", expected \"#RRGGBB\""));
		}
		let channel_at = |start_index: usize| {
			u8::from_str_radix(&hex_digits[start_index..start_index + 2], 16)
				.expect("validated hex digits")
		};
		Ok(Self { r: channel_at(0), g: channel_at(2), b: channel_at(4) })
	}
}

impl fmt::Display for Color {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
	}
}

impl TryFrom<String> for Color {
	type Error = String;

	fn try_from(text: String) -> Result<Self, String> {
		text.parse()
	}
}

impl From<Color> for String {
	fn from(color: Color) -> Self {
		color.to_string()
	}
}

pub fn load() -> (Config, Option<String>) {
	let config_path = match config_file_path() {
		Ok(config_path) => config_path,
		Err(error) => return (Config::default(), Some(error)),
	};
	let file_bytes = match fs::read(&config_path) {
		Ok(file_bytes) => file_bytes,
		Err(error) if error.kind() == io::ErrorKind::NotFound => {
			return (Config::default(), None);
		},
		Err(error) => {
			return (Config::default(), Some(load_failed_message(&config_path, error)));
		},
	};
	let Some(file_json) =
		serde_json::from_slice(&file_bytes).ok().filter(Value::is_object)
	else {
		return (Config::default(), Some(back_up_unreadable_file(&config_path)));
	};
	let config = config_from_json(&file_json);
	let normalized_json = serde_json::to_value(&config).expect("config serializes");
	let warning = (normalized_json != file_json)
		.then(|| save(&config).err())
		.flatten()
		.map(|error| format!("Failed to update settings.\n\n{error}"));
	(config, warning)
}

fn config_from_json(stored_json: &Value) -> Config {
	let default_profile = Profile {
		name: DEFAULT_PROFILE_NAME.into(),
		crosshair: crosshair_from_json(&stored_json["default"]),
	};
	let stored_profiles =
		stored_json["profiles"].as_array().map_or(&[][..], Vec::as_slice);
	Config {
		active_profile_name: stored_json["active"]
			.as_str()
			.unwrap_or(DEFAULT_PROFILE_NAME)
			.into(),
		uses_windows_scaling: stored_json["windows_scaling"].as_bool().unwrap_or(true),
		profiles: iter::once(default_profile)
			.chain(stored_profiles.iter().filter_map(|stored_profile| {
				Some(Profile {
					name: stored_profile["name"].as_str()?.into(),
					crosshair: crosshair_from_json(&stored_profile["crosshair"]),
				})
			}))
			.collect(),
	}
	.normalized()
}

fn crosshair_from_json(stored_json: &Value) -> Crosshair {
	let mut merged_settings =
		serde_json::to_value(Crosshair::default()).expect("crosshair serializes");
	for leaf_pointer in leaf_pointers(&merged_settings, String::new()) {
		let Some(stored_value) = stored_json.pointer(&leaf_pointer) else { continue };
		let mut settings_with_stored_value = merged_settings.clone();
		*settings_with_stored_value.pointer_mut(&leaf_pointer).expect("leaf exists") =
			stored_value.clone();
		if serde_json::from_value::<Crosshair>(settings_with_stored_value.clone())
			.is_ok()
		{
			merged_settings = settings_with_stored_value;
		}
	}
	serde_json::from_value(merged_settings).expect("merged settings are valid")
}

fn leaf_pointers(json_value: &Value, pointer_prefix: String) -> Vec<String> {
	match json_value {
		Value::Object(object) => object
			.iter()
			.flat_map(|(key, child_value)| {
				leaf_pointers(child_value, format!("{pointer_prefix}/{key}"))
			})
			.collect(),
		_ => vec![pointer_prefix],
	}
}

fn back_up_unreadable_file(path: &Path) -> String {
	let backup_path = (0..)
		.map(|backup_number| match backup_number {
			0 => path.with_file_name("config.bak.json"),
			backup_number => {
				path.with_file_name(format!("config.bak.{backup_number}.json"))
			},
		})
		.find(|backup_path| !backup_path.exists())
		.expect("a free backup name exists");
	match fs::rename(path, &backup_path) {
		Ok(()) => format!(
			"Your settings file couldn't be read, so defaults are used.\n\nThe old file was moved to {}.",
			backup_path.display()
		),
		Err(error) => load_failed_message(path, error),
	}
}

fn load_failed_message(path: &Path, error: io::Error) -> String {
	format!("Failed to load settings, using defaults.\n\n{}: {error}", path.display())
}

pub fn save(config: &Config) -> Result<(), String> {
	let config_path = config_file_path()?;
	let mut pretty_json = Vec::new();
	let formatter = PrettyFormatter::with_indent(b"\t");
	config
		.serialize(&mut Serializer::with_formatter(&mut pretty_json, formatter))
		.map_err(|error| error.to_string())?;
	// Writing a temporary file and renaming it over the old one means a crash
	// mid-write can't leave a truncated settings file behind.
	let temporary_path = config_path.with_extension("json.tmp");
	fs::create_dir_all(config_path.parent().expect("config path has a parent"))
		.and_then(|()| fs::write(&temporary_path, pretty_json))
		.and_then(|()| fs::rename(&temporary_path, &config_path))
		.map_err(|error| format!("{}: {error}", config_path.display()))
}

fn config_file_path() -> Result<PathBuf, String> {
	let local_app_data =
		std::env::var_os("LOCALAPPDATA").ok_or("LOCALAPPDATA is not set")?;
	Ok(PathBuf::from(local_app_data).join("deadeye").join("config.json"))
}

#[cfg(test)]
mod tests {
	use serde_json::json;

	use super::*;

	#[test]
	fn colors_parse_with_or_without_hash_and_print_uppercase() {
		let color: Color = "00ff7f".parse().unwrap();
		assert_eq!(color, "#00FF7F".parse().unwrap());
		assert_eq!(color.to_string(), "#00FF7F");
		for invalid in ["", "#12345", "#1234567", "#GG0000", "#00ff7f "] {
			assert!(invalid.parse::<Color>().is_err(), "{invalid:?}");
		}
	}

	#[test]
	fn invalid_settings_fall_back_one_field_at_a_time() {
		let config = config_from_json(&json!({
			"default": {
				"bars": { "length": 500, "gap": "wide", "color": "#ff0000", "shape": "Sharp" },
				"dot": { "enabled": false, "opacity": -1.0, "color": "red" },
			},
		}));
		let crosshair = config.active_profile().crosshair;
		let default = Crosshair::default();
		assert_eq!(crosshair.bars.length, *BAR_LENGTH_RANGE.end());
		assert_eq!(crosshair.bars.gap, default.bars.gap);
		assert_eq!(crosshair.bars.color, Color { r: 0xFF, g: 0, b: 0 });
		assert_eq!(crosshair.bars.shape, Shape::Sharp);
		assert!(!crosshair.dot.enabled);
		assert_eq!(crosshair.dot.opacity, 0.0);
		assert_eq!(crosshair.dot.color, default.dot.color);
	}

	#[test]
	fn profiles_are_deduplicated_and_the_active_name_resolved() {
		let config = config_from_json(&json!({
			"active": "MAIN",
			"profiles": [
				{ "name": "Main", "crosshair": { "dot": { "size": 9 } } },
				{ "name": "main" },
				{ "name": "DEFAULT" },
				{ "name": "  " },
				{ "crosshair": {} },
				{ "name": "Other" },
			],
		}));
		let names: Vec<&str> =
			config.profiles.iter().map(|profile| profile.name.as_str()).collect();
		assert_eq!(names, [DEFAULT_PROFILE_NAME, "Main", "Other"]);
		assert_eq!(config.active_profile_name, "Main");
		assert_eq!(config.active_profile().crosshair.dot.size, 9);
	}

	#[test]
	fn an_unknown_active_profile_falls_back_to_the_default() {
		let config =
			config_from_json(&json!({ "active": "Gone", "windows_scaling": false }));
		assert_eq!(config.active_profile_name, DEFAULT_PROFILE_NAME);
		assert!(!config.uses_windows_scaling);
		assert!(Config::default().uses_windows_scaling);
	}

	#[test]
	fn saved_settings_load_back_unchanged() {
		let mut config = config_from_json(&json!({
			"active": "Wide",
			"windows_scaling": false,
			"profiles": [{ "name": "Wide", "crosshair": { "bars": { "gap": 30 } } }],
		}));
		config.active_profile_mut().crosshair.dot.color = Color { r: 1, g: 2, b: 3 };
		let saved = serde_json::to_value(&config).unwrap();
		assert_eq!(saved["version"], FILE_FORMAT_VERSION);
		assert_eq!(serde_json::to_value(config_from_json(&saved)).unwrap(), saved);
	}
}
