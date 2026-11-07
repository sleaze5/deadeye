use std::cell::RefCell;
use std::ptr;

use crate::config::{
	self, Config, Crosshair, DEFAULT_PROFILE_NAME, Profile, names_match_ignoring_case,
};
use crate::win;

pub const MAX_PROFILE_NAME_LENGTH: usize = 32;

thread_local! {
	static LOADED_CONFIG: RefCell<Config> = RefCell::new(Config::default());
}

pub fn load_from_disk() {
	let (config, warning) = config::load();
	if let Some(warning) = warning {
		win::show_error(ptr::null_mut(), &warning);
	}
	LOADED_CONFIG.set(config);
}

pub fn profile_names() -> Vec<String> {
	LOADED_CONFIG.with_borrow(|config| {
		config.profiles.iter().map(|profile| profile.name.clone()).collect()
	})
}

pub fn active_profile_name() -> String {
	LOADED_CONFIG.with_borrow(|config| config.active_profile().name.clone())
}

pub fn is_default_profile_active() -> bool {
	LOADED_CONFIG.with_borrow(Config::is_default_profile_active)
}

pub fn saved_crosshair() -> Crosshair {
	LOADED_CONFIG.with_borrow(|config| config.active_profile().crosshair)
}

pub fn uses_windows_scaling() -> bool {
	LOADED_CONFIG.with_borrow(|config| config.uses_windows_scaling)
}

pub fn set_uses_windows_scaling(enabled: bool) -> Result<(), String> {
	update_and_save(|config| {
		config.uses_windows_scaling = enabled;
		Ok(())
	})
}

pub fn switch_to_profile(profile_name: &str) -> Result<Crosshair, String> {
	update_and_save(|config| {
		config.active_profile_name = profile_name.into();
		Ok(())
	})?;
	Ok(saved_crosshair())
}

pub fn save_active_profile(crosshair: Crosshair) -> Result<(), String> {
	update_and_save(|config| {
		config.active_profile_mut().crosshair = crosshair;
		Ok(())
	})
}

pub fn create_profile(
	requested_name: &str,
	crosshair: Crosshair,
) -> Result<(), String> {
	update_and_save(|config| {
		let valid_name = validated_profile_name(config, requested_name, None)?;
		config.profiles.push(Profile { name: valid_name.clone(), crosshair });
		config.active_profile_name = valid_name;
		Ok(())
	})
}

pub fn rename_active_profile(requested_name: &str) -> Result<(), String> {
	update_and_save(|config| {
		ensure_default_profile_not_active(config, "renamed")?;
		let current_name = config.active_profile_name.clone();
		let valid_name =
			validated_profile_name(config, requested_name, Some(&current_name))?;
		config.active_profile_mut().name = valid_name.clone();
		config.active_profile_name = valid_name;
		Ok(())
	})
}

pub fn delete_active_profile() -> Result<Crosshair, String> {
	update_and_save(|config| {
		ensure_default_profile_not_active(config, "deleted")?;
		let deleted_name = config.active_profile_name.clone();
		config
			.profiles
			.retain(|profile| !names_match_ignoring_case(&profile.name, &deleted_name));
		config.active_profile_name = DEFAULT_PROFILE_NAME.into();
		Ok(())
	})?;
	Ok(saved_crosshair())
}

fn update_and_save(
	apply_change: impl FnOnce(&mut Config) -> Result<(), String>,
) -> Result<(), String> {
	let mut updated_config = LOADED_CONFIG.with_borrow(Config::clone);
	apply_change(&mut updated_config)?;
	config::save(&updated_config)?;
	LOADED_CONFIG.set(updated_config);
	Ok(())
}

fn ensure_default_profile_not_active(
	config: &Config,
	past_tense_action: &str,
) -> Result<(), String> {
	if config.is_default_profile_active() {
		return Err(format!(
			"The {DEFAULT_PROFILE_NAME} profile can't be {past_tense_action}."
		));
	}
	Ok(())
}

fn validated_profile_name(
	config: &Config,
	requested_name: &str,
	current_name_when_renaming: Option<&str>,
) -> Result<String, String> {
	let trimmed_name = requested_name.trim();
	if trimmed_name.is_empty() {
		return Err("Enter a profile name.".into());
	}
	if names_match_ignoring_case(trimmed_name, DEFAULT_PROFILE_NAME) {
		return Err(format!("\"{DEFAULT_PROFILE_NAME}\" is a reserved profile name."));
	}
	if trimmed_name.chars().count() > MAX_PROFILE_NAME_LENGTH {
		return Err(format!(
			"Profile names can be at most {MAX_PROFILE_NAME_LENGTH} characters."
		));
	}
	let is_taken_by_another_profile = config.profiles.iter().any(|profile| {
		names_match_ignoring_case(&profile.name, trimmed_name)
			&& current_name_when_renaming.is_none_or(|current_name| {
				!names_match_ignoring_case(current_name, trimmed_name)
			})
	});
	if is_taken_by_another_profile {
		return Err(format!("A profile named \"{trimmed_name}\" already exists."));
	}
	Ok(trimmed_name.into())
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
		assert_eq!(validated_profile_name(&config, "  Wide ", None).unwrap(), "Wide");
		assert!(validated_profile_name(&config, "MAIN", None).is_err());
		assert!(validated_profile_name(&config, "default", None).is_err());
		assert!(validated_profile_name(&config, " ", None).is_err());
		assert!(
			validated_profile_name(&config, &"x".repeat(MAX_PROFILE_NAME_LENGTH), None)
				.is_ok()
		);
		assert!(
			validated_profile_name(
				&config,
				&"x".repeat(MAX_PROFILE_NAME_LENGTH + 1),
				None
			)
			.is_err()
		);
	}

	#[test]
	fn a_profile_can_be_renamed_to_a_different_case_of_its_own_name() {
		let mut config = config();
		config
			.profiles
			.push(Profile { name: "Other".into(), crosshair: Crosshair::default() });
		assert_eq!(
			validated_profile_name(&config, "MAIN", Some("Main")).unwrap(),
			"MAIN"
		);
		assert!(validated_profile_name(&config, "other", Some("Main")).is_err());
	}
}
