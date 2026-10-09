use std::path::PathBuf;
use std::{env, fs};

fn main() {
	println!("cargo:rerun-if-changed=assets");
	println!("cargo:rerun-if-changed=Cargo.toml");
	embed_resource::compile("assets/deadeye.rc", embed_resource::NONE)
		.manifest_required()
		.unwrap();
	embed_resource::compile(version_resource(), embed_resource::NONE)
		.manifest_optional()
		.unwrap();
}

fn version_resource() -> PathBuf {
	let version = env::var("CARGO_PKG_VERSION").unwrap();
	let numbers = version
		.split(['.', '-', '+'])
		.take(3)
		.chain(["0"])
		.collect::<Vec<_>>()
		.join(",");
	let description = env::var("CARGO_PKG_DESCRIPTION").unwrap();
	let authors = env::var("CARGO_PKG_AUTHORS").unwrap();
	let rc = format!(
		r#"#include <windows.h>

VS_VERSION_INFO VERSIONINFO
FILEVERSION {numbers}
PRODUCTVERSION {numbers}
FILEOS VOS_NT_WINDOWS32
FILETYPE VFT_APP
BEGIN
	BLOCK "StringFileInfo"
	BEGIN
		BLOCK "040904B0"
		BEGIN
			VALUE "CompanyName", "{authors}"
			VALUE "FileDescription", "{description}"
			VALUE "FileVersion", "{version}"
			VALUE "InternalName", "deadeye"
			VALUE "LegalCopyright", "Copyright (c) 2026 {authors}"
			VALUE "OriginalFilename", "deadeye.exe"
			VALUE "ProductName", "Deadeye"
			VALUE "ProductVersion", "{version}"
		END
	END
	BLOCK "VarFileInfo"
	BEGIN
		VALUE "Translation", 0x409, 1200
	END
END
"#
	);
	let path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("version.rc");
	fs::write(&path, rc).unwrap();
	path
}
