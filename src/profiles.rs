use std::cell::RefCell;
use std::ptr;

use crate::config::{self, Config, Crosshair, DEFAULT_PROFILE, Profile, same_name};
use crate::win;

pub const MAX_NAME_LENGTH: usize = 32;

thread_local! {
	static CONFIG: RefCell<Config> = RefCell::new(Config::default());
}

pub fn load() {
	let (config, warning) = config::load();
	if let Some(warning) = warning {
		win::show_error(ptr::null_mut(), &warning);
	}
	CONFIG.set(config);
}

pub fn names() -> Vec<String> {
	CONFIG.with_borrow(|config| {
		config.profiles.iter().map(|profile| profile.name.clone()).collect()
	})
}

pub fn active() -> String {
	CONFIG.with_borrow(|config| config.active().name.clone())
}

pub fn is_default() -> bool {
	same_name(&active(), DEFAULT_PROFILE)
}

pub fn saved() -> Crosshair {
	CONFIG.with_borrow(|config| config.active().crosshair)
}

pub fn windows_scaling() -> bool {
	CONFIG.with_borrow(|config| config.windows_scaling)
}

pub fn set_windows_scaling(on: bool) -> Result<(), String> {
	update(|config| {
		config.windows_scaling = on;
		Ok(())
	})
}

pub fn select(name: &str) -> Result<Crosshair, String> {
	update(|config| {
		config.active = name.into();
		Ok(())
	})?;
	Ok(saved())
}

pub fn save(crosshair: Crosshair) -> Result<(), String> {
	update(|config| {
		config.active_mut().crosshair = crosshair;
		Ok(())
	})
}

pub fn create(name: &str, crosshair: Crosshair) -> Result<(), String> {
	update(|config| {
		let name = valid_name(config, name, None)?;
		config.profiles.push(Profile { name: name.clone(), crosshair });
		config.active = name;
		Ok(())
	})
}

pub fn rename(name: &str) -> Result<(), String> {
	update(|config| {
		ensure_not_default(config, "renamed")?;
		let current = config.active.clone();
		let name = valid_name(config, name, Some(&current))?;
		config.active_mut().name = name.clone();
		config.active = name;
		Ok(())
	})
}

pub fn delete() -> Result<Crosshair, String> {
	update(|config| {
		ensure_not_default(config, "deleted")?;
		let current = config.active.clone();
		config.profiles.retain(|profile| !same_name(&profile.name, &current));
		config.active = DEFAULT_PROFILE.into();
		Ok(())
	})?;
	Ok(saved())
}

fn update(
	change: impl FnOnce(&mut Config) -> Result<(), String>,
) -> Result<(), String> {
	let mut next = CONFIG.with_borrow(Config::clone);
	change(&mut next)?;
	config::save(&next)?;
	CONFIG.set(next);
	Ok(())
}

fn ensure_not_default(config: &Config, action: &str) -> Result<(), String> {
	if same_name(&config.active, DEFAULT_PROFILE) {
		return Err(format!("The {DEFAULT_PROFILE} profile can't be {action}."));
	}
	Ok(())
}

fn valid_name(
	config: &Config,
	name: &str,
	renaming: Option<&str>,
) -> Result<String, String> {
	let name = name.trim();
	if name.is_empty() {
		return Err("Enter a profile name.".into());
	}
	if same_name(name, DEFAULT_PROFILE) {
		return Err(format!("\"{DEFAULT_PROFILE}\" is a reserved profile name."));
	}
	if name.chars().count() > MAX_NAME_LENGTH {
		return Err(format!(
			"Profile names can be at most {MAX_NAME_LENGTH} characters."
		));
	}
	let taken = config.profiles.iter().any(|profile| {
		same_name(&profile.name, name)
			&& renaming.is_none_or(|current| !same_name(current, name))
	});
	if taken {
		return Err(format!("A profile named \"{name}\" already exists."));
	}
	Ok(name.into())
}

#[cfg(test)]
mod tests {
	use super::*;

	fn config() -> Config {
		let mut config = Config::default();
		config
			.profiles
			.push(Profile { name: "Main".into(), crosshair: Crosshair::default() });
		config
	}

	#[test]
	fn new_names_are_trimmed_and_must_be_unique() {
		let config = config();
		assert_eq!(valid_name(&config, "  Wide ", None).unwrap(), "Wide");
		assert!(valid_name(&config, "MAIN", None).is_err());
		assert!(valid_name(&config, "default", None).is_err());
		assert!(valid_name(&config, " ", None).is_err());
		assert!(valid_name(&config, &"x".repeat(MAX_NAME_LENGTH), None).is_ok());
		assert!(valid_name(&config, &"x".repeat(MAX_NAME_LENGTH + 1), None).is_err());
	}

	#[test]
	fn a_profile_can_be_renamed_to_a_different_case_of_its_own_name() {
		let mut config = config();
		config
			.profiles
			.push(Profile { name: "Other".into(), crosshair: Crosshair::default() });
		assert_eq!(valid_name(&config, "MAIN", Some("Main")).unwrap(), "MAIN");
		assert!(valid_name(&config, "other", Some("Main")).is_err());
	}
}
