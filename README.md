# Deadeye

Deadeye is a lightweight crosshair overlay for Windows. You can customize the crosshair to look the way you want. Deadeye runs quietly in the system tray.

> [!IMPORTANT]\
> It requires Windows 10 version 1607 or higher, or Windows 11.

## Usage

1. Run `deadeye.exe`
2. Click the deadeye tray icon to open the customization window

> [!NOTE]\
> Deadeye saves your settings in `%LOCALAPPDATA%\deadeye\config.json`.

## Building

### Prerequisites

- [Rust](https://rustup.rs) 1.85 or later with the MSVC toolchain (`x86_64-pc-windows-msvc`)
- [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with the **Desktop development with C++** workload, which includes the Windows SDK

The build embeds the icon, manifest, dialogs, and version information into the executable. This step uses the resource compiler (`rc.exe`) from the Windows SDK.

### Build

```sh
cargo build --release
```

The executable is created at `target\release\deadeye.exe`.

## Development

Run `cargo xtask` to see the available development tasks.

## License

Deadeye is licensed under the [MIT License](LICENSE).
