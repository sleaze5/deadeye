use std::env;
use std::error::Error;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use ico::{IconDir, IconDirEntry, IconImage, ResourceType};
use image::RgbaImage;
use image::imageops::{self, FilterType};
use resvg::{tiny_skia, usvg};

const SIZES: [u32; 15] = [16, 20, 24, 30, 32, 36, 40, 48, 60, 64, 72, 80, 96, 128, 256];
const RENDER_SIZE: u32 = SIZES[SIZES.len() - 1] * 4;
const USAGE: &str = "Usage: cargo xtask <task>

Tasks:
  check             Run lints and static analysis
  check --fix       Apply automatic lint fixes, then run check
  fix               Apply automatic lint fixes, format the code, then run check
  format            Format the code
  format --check    Check the formatting without changing anything
  icon              Generate assets/icon.ico from assets/icon.svg";

type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

fn main() -> ExitCode {
	let args: Vec<String> = env::args().skip(1).collect();
	let args: Vec<&str> = args.iter().map(String::as_str).collect();
	let result = match args.as_slice() {
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

fn check(fix: bool) -> Result {
	if fix {
		apply_lint_fixes()?;
	}
	lint()
}

fn fix() -> Result {
	apply_lint_fixes()?;
	format(false)?;
	lint()
}

fn apply_lint_fixes() -> Result {
	cargo(&[
		"clippy",
		"--workspace",
		"--all-targets",
		"--fix",
		"--allow-dirty",
		"--allow-staged",
	])
}

fn lint() -> Result {
	cargo(&[
		"clippy",
		"--workspace",
		"--all-targets",
		"--locked",
		"--",
		"-D",
		"warnings",
	])
}

fn format(check: bool) -> Result {
	let mut args = vec!["fmt", "--all"];
	if check {
		args.push("--check");
	}
	cargo(&args)
}

fn cargo(args: &[&str]) -> Result {
	let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
	let status = Command::new(cargo).args(args).current_dir(root()).status()?;
	if !status.success() {
		return Err(format!("cargo {} failed", args.join(" ")).into());
	}
	Ok(())
}

fn icon() -> Result {
	let assets = root().join("assets");
	let source = assets.join("icon.svg");
	let output = assets.join("icon.ico");

	let master = render(&source, RENDER_SIZE)?;
	let mut icon = IconDir::new(ResourceType::Icon);
	for size in SIZES {
		let frame =
			unpremultiply(imageops::resize(&master, size, size, FilterType::Lanczos3));
		let image = IconImage::from_rgba_data(size, size, frame.into_raw());
		icon.add_entry(IconDirEntry::encode_as_png(&image)?);
	}
	icon.write(File::create(&output)?)?;
	verify(&output)?;

	println!("Created {}", output.display());
	println!("Source render: {RENDER_SIZE}x{RENDER_SIZE}");
	println!("Embedded sizes: {SIZES:?}");
	Ok(())
}

fn root() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR"))
		.parent()
		.expect("xtask is inside the workspace")
		.to_path_buf()
}

fn render(path: &Path, size: u32) -> Result<RgbaImage> {
	let tree = usvg::Tree::from_data(&fs::read(path)?, &usvg::Options::default())?;
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
		let [r, g, b, a] = pixel.0;
		let channel = |value: u8| {
			let value = u32::from(value.min(a));
			((value * 255 + u32::from(a) / 2) / u32::from(a)) as u8
		};
		pixel.0 = if a == 0 { [0; 4] } else { [channel(r), channel(g), channel(b), a] };
	}
	image
}

fn verify(path: &Path) -> Result {
	let icon = IconDir::read(File::open(path)?)?;
	let mut actual: Vec<(u32, u32)> =
		icon.entries().iter().map(|entry| (entry.width(), entry.height())).collect();
	actual.sort_unstable();
	let expected: Vec<(u32, u32)> = SIZES.iter().map(|&size| (size, size)).collect();
	if actual != expected {
		return Err(format!("expected sizes {expected:?}, found {actual:?}").into());
	}
	Ok(())
}
