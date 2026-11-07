use std::env;
use std::error::Error;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use ico::{IconDir, IconDirEntry, IconImage, ResourceType};
use image::RgbaImage;
use image::imageops::{self, FilterType};
use resvg::{tiny_skia, usvg};

const ICON_SIZES: [u32; 15] =
	[16, 20, 24, 30, 32, 36, 40, 48, 60, 64, 72, 80, 96, 128, 256];
const SOURCE_RENDER_SIZE: u32 = ICON_SIZES[ICON_SIZES.len() - 1] * 4;
const USAGE: &str = "Usage: cargo xtask <task>

Tasks:
  check             Run lints, static analysis and tests
  check --fix       Apply automatic lint fixes, then run check
  fix               Apply automatic lint fixes, format the code, then run check
  format            Format the code
  format --check    Check the formatting without changing anything
  icon              Generate assets/icon.ico from assets/icon.svg";

type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

fn main() -> ExitCode {
	let arguments: Vec<String> = env::args().skip(1).collect();
	let argument_slices: Vec<&str> = arguments.iter().map(String::as_str).collect();
	let result = match argument_slices.as_slice() {
		["check"] => check(false),
		["check", "--fix"] => check(true),
		["fix"] => fix(),
		["format"] => format(false),
		["format", "--check"] => format(true),
		["icon"] => icon(),
		_ => Err(USAGE.into()),
	};
	match result {
		Ok(()) => ExitCode::SUCCESS,
		Err(e) => {
			eprintln!("{e}");
			ExitCode::FAILURE
		},
	}
}

fn check(apply_fixes_first: bool) -> Result {
	if apply_fixes_first {
		apply_lint_fixes()?;
	}
	run_lints()?;
	run_cargo(&["test", "--workspace", "--locked"])
}

fn fix() -> Result {
	apply_lint_fixes()?;
	format(false)?;
	check(false)
}

fn apply_lint_fixes() -> Result {
	run_cargo(&[
		"clippy",
		"--workspace",
		"--all-targets",
		"--fix",
		"--allow-dirty",
		"--allow-staged",
	])
}

fn run_lints() -> Result {
	run_cargo(&[
		"clippy",
		"--workspace",
		"--all-targets",
		"--locked",
		"--",
		"-D",
		"warnings",
	])
}

fn format(check_only: bool) -> Result {
	let mut fmt_arguments = vec!["fmt", "--all"];
	if check_only {
		fmt_arguments.push("--check");
	}
	run_cargo(&fmt_arguments)
}

fn run_cargo(args: &[&str]) -> Result {
	let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
	let status =
		Command::new(cargo).args(args).current_dir(workspace_root()).status()?;
	if !status.success() {
		return Err(format!("cargo {} failed", args.join(" ")).into());
	}
	Ok(())
}

fn icon() -> Result {
	let assets = workspace_root().join("assets");
	let source = assets.join("icon.svg");
	let output = assets.join("icon.ico");

	let source_render = render_svg(&source, SOURCE_RENDER_SIZE)?;
	let mut icon = IconDir::new(ResourceType::Icon);
	for size in ICON_SIZES {
		let resized_frame = unpremultiply(imageops::resize(
			&source_render,
			size,
			size,
			FilterType::Lanczos3,
		));
		let image = IconImage::from_rgba_data(size, size, resized_frame.into_raw());
		icon.add_entry(IconDirEntry::encode_as_png(&image)?);
	}
	icon.write(File::create(&output)?)?;
	verify_icon_sizes(&output)?;

	println!("Created {}", output.display());
	println!("Source render: {SOURCE_RENDER_SIZE}x{SOURCE_RENDER_SIZE}");
	println!("Embedded sizes: {ICON_SIZES:?}");
	Ok(())
}

fn workspace_root() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR"))
		.parent()
		.expect("xtask is inside the workspace")
		.to_path_buf()
}

fn render_svg(svg_path: &Path, size: u32) -> Result<RgbaImage> {
	let tree = usvg::Tree::from_data(&fs::read(svg_path)?, &usvg::Options::default())?;
	let mut pixmap = tiny_skia::Pixmap::new(size, size).ok_or("invalid render size")?;
	let scale_x = size as f32 / tree.size().width();
	let scale_y = size as f32 / tree.size().height();
	let transform = tiny_skia::Transform::from_scale(scale_x, scale_y);
	resvg::render(&tree, transform, &mut pixmap.as_mut());
	RgbaImage::from_raw(size, size, pixmap.take())
		.ok_or_else(|| "invalid render buffer".into())
}

fn unpremultiply(mut image: RgbaImage) -> RgbaImage {
	for pixel in image.pixels_mut() {
		let [red, green, blue, alpha] = pixel.0;
		let unpremultiplied_channel = |value: u8| {
			let value = u32::from(value.min(alpha));
			((value * 255 + u32::from(alpha) / 2) / u32::from(alpha)) as u8
		};
		pixel.0 = if alpha == 0 {
			[0; 4]
		} else {
			[
				unpremultiplied_channel(red),
				unpremultiplied_channel(green),
				unpremultiplied_channel(blue),
				alpha,
			]
		};
	}
	image
}

fn verify_icon_sizes(icon_path: &Path) -> Result {
	let icon = IconDir::read(File::open(icon_path)?)?;
	let mut actual_sizes: Vec<(u32, u32)> =
		icon.entries().iter().map(|entry| (entry.width(), entry.height())).collect();
	actual_sizes.sort_unstable();
	let expected_sizes: Vec<(u32, u32)> =
		ICON_SIZES.iter().map(|&size| (size, size)).collect();
	if actual_sizes != expected_sizes {
		return Err(format!(
			"expected sizes {expected_sizes:?}, found {actual_sizes:?}"
		)
		.into());
	}
	Ok(())
}
