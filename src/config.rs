use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::{fmt, fs, io, iter};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_json::ser::{PrettyFormatter, Serializer};

pub const LENGTH_RANGE: RangeInclusive<i32> = 1..=50;
pub const THICKNESS_RANGE: RangeInclusive<i32> = 1..=20;
pub const GAP_RANGE: RangeInclusive<i32> = 0..=50;
pub const DOT_SIZE_RANGE: RangeInclusive<i32> = 1..=30;
pub const OUTLINE_RANGE: RangeInclusive<i32> = 1..=10;

pub const DEFAULT_PROFILE: &str = "Default";

const VERSION: u32 = 1;

#[derive(Clone)]
pub struct Config {
	pub active: String,
	pub windows_scaling: bool,
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
		from_json(&Value::Null)
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
			version: VERSION,
			active: &self.active,
			windows_scaling: self.windows_scaling,
			default: default.crosshair,
			profiles,
		}
		.serialize(serializer)
	}
}

impl Config {
	pub fn profile(&self, name: &str) -> Option<&Profile> {
		self.profiles.iter().find(|profile| same_name(&profile.name, name))
	}

	pub fn active(&self) -> &Profile {
		self.profile(&self.active).expect("the active profile exists")
	}

	pub fn active_mut(&mut self) -> &mut Profile {
		let active = self.active.clone();
		self.profiles
			.iter_mut()
			.find(|profile| same_name(&profile.name, &active))
			.expect("the active profile exists")
	}

	fn normalized(mut self) -> Self {
		let mut names: Vec<String> = Vec::new();
		self.profiles.retain(|profile| {
			let unique = !profile.name.trim().is_empty()
				&& !names.iter().any(|name| same_name(name, &profile.name));
			names.push(profile.name.clone());
			unique
		});
		for profile in &mut self.profiles {
			profile.crosshair = profile.crosshair.clamped();
		}
		self.active = self
			.profile(&self.active)
			.map_or(DEFAULT_PROFILE.into(), |profile| profile.name.clone());
		self
	}
}

pub fn same_name(a: &str, b: &str) -> bool {
	a.to_lowercase() == b.to_lowercase()
}

impl Crosshair {
	pub fn clamped(self) -> Self {
		Self {
			bars: Bars {
				length: clamp(self.bars.length, &LENGTH_RANGE),
				thickness: clamp(self.bars.thickness, &THICKNESS_RANGE),
				gap: clamp(self.bars.gap, &GAP_RANGE),
				opacity: self.bars.opacity.clamp(0.0, 1.0),
				outline: self.bars.outline.clamped(),
				..self.bars
			},
			dot: Dot {
				size: clamp(self.dot.size, &DOT_SIZE_RANGE),
				opacity: self.dot.opacity.clamp(0.0, 1.0),
				outline: self.dot.outline.clamped(),
				..self.dot
			},
		}
	}
}

impl Bars {
	pub fn any(&self) -> bool {
		self.top || self.bottom || self.left || self.right
	}
}

impl Outline {
	fn clamped(self) -> Self {
		Self {
			thickness: clamp(self.thickness, &OUTLINE_RANGE),
			opacity: self.opacity.clamp(0.0, 1.0),
			..self
		}
	}
}

fn clamp(value: i32, range: &RangeInclusive<i32>) -> i32 {
	value.clamp(*range.start(), *range.end())
}

impl FromStr for Color {
	type Err = String;

	fn from_str(text: &str) -> Result<Self, String> {
		let hex = text.strip_prefix('#').unwrap_or(text);
		if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
			return Err(format!("invalid color \"{text}\", expected \"#RRGGBB\""));
		}
		let channel = |i: usize| {
			u8::from_str_radix(&hex[i..i + 2], 16).expect("validated hex digits")
		};
		Ok(Self { r: channel(0), g: channel(2), b: channel(4) })
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
	let path = match path() {
		Ok(path) => path,
		Err(e) => return (Config::default(), Some(e)),
	};
	let json = match fs::read_to_string(&path) {
		Ok(text) => serde_json::from_str::<Value>(&text).ok().filter(Value::is_object),
		Err(e) if e.kind() == io::ErrorKind::NotFound => {
			return (Config::default(), None);
		},
		Err(e) if e.kind() == io::ErrorKind::InvalidData => None,
		Err(e) => {
			let message = format!(
				"Failed to load settings, using defaults.\n\n{}: {e}",
				path.display()
			);
			return (Config::default(), Some(message));
		},
	};
	let Some(json) = json else {
		return (Config::default(), Some(back_up(&path)));
	};
	let config = from_json(&json);
	let cleaned = serde_json::to_value(&config).expect("config serializes");
	let warning = (cleaned != json)
		.then(|| save(&config).err())
		.flatten()
		.map(|e| format!("Failed to update settings.\n\n{e}"));
	(config, warning)
}

fn from_json(json: &Value) -> Config {
	let default = Profile {
		name: DEFAULT_PROFILE.into(),
		crosshair: crosshair_from_json(&json["default"]),
	};
	let profiles = json["profiles"].as_array().map_or(&[][..], Vec::as_slice);
	Config {
		active: json["active"].as_str().unwrap_or(DEFAULT_PROFILE).into(),
		windows_scaling: json["windows_scaling"].as_bool().unwrap_or(true),
		profiles: iter::once(default)
			.chain(profiles.iter().filter_map(|profile| {
				Some(Profile {
					name: profile["name"].as_str()?.into(),
					crosshair: crosshair_from_json(&profile["crosshair"]),
				})
			}))
			.collect(),
	}
	.normalized()
}

fn crosshair_from_json(json: &Value) -> Crosshair {
	let mut merged =
		serde_json::to_value(Crosshair::default()).expect("crosshair serializes");
	for path in leaves(&merged, String::new()) {
		let Some(value) = json.pointer(&path) else { continue };
		let mut candidate = merged.clone();
		*candidate.pointer_mut(&path).expect("leaf exists") = value.clone();
		if serde_json::from_value::<Crosshair>(candidate.clone()).is_ok() {
			merged = candidate;
		}
	}
	serde_json::from_value(merged).expect("merged settings are valid")
}

fn leaves(value: &Value, path: String) -> Vec<String> {
	match value {
		Value::Object(map) => map
			.iter()
			.flat_map(|(key, value)| leaves(value, format!("{path}/{key}")))
			.collect(),
		_ => vec![path],
	}
}

fn back_up(path: &Path) -> String {
	let backup = (0..)
		.map(|n| match n {
			0 => path.with_file_name("config.bak.json"),
			n => path.with_file_name(format!("config.bak.{n}.json")),
		})
		.find(|backup| !backup.exists())
		.expect("a free backup name exists");
	match fs::rename(path, &backup) {
		Ok(()) => format!(
			"Your settings file couldn't be read, so defaults are used.\n\nThe old file was moved to {}.",
			backup.display()
		),
		Err(e) => format!(
			"Failed to load settings, using defaults.\n\n{}: {e}",
			path.display()
		),
	}
}

pub fn save(config: &Config) -> Result<(), String> {
	let path = path()?;
	let mut json = Vec::new();
	let formatter = PrettyFormatter::with_indent(b"\t");
	config
		.serialize(&mut Serializer::with_formatter(&mut json, formatter))
		.map_err(|e| e.to_string())?;
	fs::create_dir_all(path.parent().expect("config path has a parent"))
		.and_then(|()| fs::write(&path, json))
		.map_err(|e| format!("{}: {e}", path.display()))
}

fn path() -> Result<PathBuf, String> {
	let local = std::env::var_os("LOCALAPPDATA").ok_or("LOCALAPPDATA is not set")?;
	Ok(PathBuf::from(local).join("deadeye").join("config.json"))
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
		let config = from_json(&json!({
			"default": {
				"bars": { "length": 500, "gap": "wide", "color": "#ff0000", "shape": "Sharp" },
				"dot": { "enabled": false, "opacity": -1.0, "color": "red" },
			},
		}));
		let crosshair = config.active().crosshair;
		let default = Crosshair::default();
		assert_eq!(crosshair.bars.length, *LENGTH_RANGE.end());
		assert_eq!(crosshair.bars.gap, default.bars.gap);
		assert_eq!(crosshair.bars.color, Color { r: 0xFF, g: 0, b: 0 });
		assert_eq!(crosshair.bars.shape, Shape::Sharp);
		assert!(!crosshair.dot.enabled);
		assert_eq!(crosshair.dot.opacity, 0.0);
		assert_eq!(crosshair.dot.color, default.dot.color);
	}

	#[test]
	fn profiles_are_deduplicated_and_the_active_name_resolved() {
		let config = from_json(&json!({
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
		assert_eq!(names, [DEFAULT_PROFILE, "Main", "Other"]);
		assert_eq!(config.active, "Main");
		assert_eq!(config.active().crosshair.dot.size, 9);
	}

	#[test]
	fn an_unknown_active_profile_falls_back_to_the_default() {
		let config = from_json(&json!({ "active": "Gone", "windows_scaling": false }));
		assert_eq!(config.active, DEFAULT_PROFILE);
		assert!(!config.windows_scaling);
		assert!(Config::default().windows_scaling);
	}

	#[test]
	fn saved_settings_load_back_unchanged() {
		let mut config = from_json(&json!({
			"active": "Wide",
			"windows_scaling": false,
			"profiles": [{ "name": "Wide", "crosshair": { "bars": { "gap": 30 } } }],
		}));
		config.active_mut().crosshair.dot.color = Color { r: 1, g: 2, b: 3 };
		let saved = serde_json::to_value(&config).unwrap();
		assert_eq!(saved["version"], VERSION);
		assert_eq!(serde_json::to_value(from_json(&saved)).unwrap(), saved);
	}
}
