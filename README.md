# Kohlrausch Konfig

**Current version: 1.2.1**

<p align="center"><img src="src-tauri/ui/assets/readme-banner.png" alt="Kohlrausch Konfig startup banner" width="760"></p>

Kohlrausch Konfig is a Windows and Linux desktop application built with Rust, Tauri 2, and a bundled HTML/CSS/JavaScript UI. On launch, a centered standalone splash window shows this banner and a five-second loading bar while the workspace starts. The earlier Slint implementation is retained in `slint-build/`; the Bun terminal prototype is in `apps/cli`. Neither is the current desktop frontend.

## Requirements

- Rust stable and Cargo
- The native build prerequisites for Tauri 2 on your operating system, including a C/C++ toolchain and the platform WebView libraries/runtime
- On Linux, install Tauri's WebKitGTK 4.1, GTK, AppIndicator, OpenSSL, and librsvg development packages before building.

The application UI is bundled from `src-tauri/ui`; there is no frontend package installation or build step for the desktop app.

## Build and launch

Run from the repository root:

```powershell
cargo build --manifest-path src-tauri/Cargo.toml --locked
cargo run --manifest-path src-tauri/Cargo.toml --locked
cargo run --manifest-path src-tauri/Cargo.toml --locked -- --config
```

The default window opens the Keymap editor. Use the sidebar to open Macros, Device settings, Config Studio, or About. Click a key in the 3D model to edit its assignment, choose one of the four layers, and drag the model to rotate it. `kohlrausch --config` opens Config Studio directly.

The root Bun scripts launch the Tauri application: `bun run start` opens the workspace, `bun run config` opens Config Studio, and `bun run kohlrausch -- device status --details` runs CLI diagnostics without opening the UI. `bun run prototype` opens the retained terminal test prototype; it is not the product frontend.

## Windows distribution builds

Build both installer formats from PowerShell:

```powershell
Push-Location .\src-tauri
try {
  cargo tauri build --bundles msi,nsis -- --locked
}
finally {
  Pop-Location
}
& .\tools\build-portable.ps1
```

The MSI and NSIS setup executable are written under `src-tauri/target/release/bundle`. They use Tauri's default WebView2 bootstrapper if the runtime is missing. The script copies the release executable into a no-install portable folder and creates an EXE and ZIP under `dist`; the portable build needs WebView2 already installed and keeps profile data in the normal user profile storage. The current release is **1.2.1**. Its Windows artifacts are `Kohlrausch Konfig_1.2.1_x64_en-US.msi`, `Kohlrausch Konfig_1.2.1_x64-setup.exe`, and `Kohlrausch-Konfig-1.2.1-portable.exe` / `.zip`.

GitHub releases are built by [`.github/workflows/release.yml`](.github/workflows/release.yml). The workflow reads the version from `src-tauri/Cargo.toml`, verifies it matches `src-tauri/tauri.conf.json`, and builds Windows MSI/NSIS installers, Windows portable EXE/ZIP, and Linux DEB/AppImage packages. Push a matching `v<version>` tag (for example, `v1.2.1`) or run the workflow manually from GitHub Actions; manual runs create the release tag from the current code version.

See [release build details](docs/releasing.md) for the version and package rules. The About page shows the Cargo package version and a 16-character build fingerprint derived from build inputs and target architecture.

Cargo places this crate's executable under `src-tauri/target` by default. The
Windows launch helpers look there for a release build first, then a debug
build; you can also launch the executable directly from either directory.

The retained helper commands are:

```powershell
.\tools\kohlrausch.cmd
.\tools\khl.cmd --c
```

## CLI commands

CLI subcommands run without starting the GUI:

```powershell
cargo run --manifest-path src-tauri/Cargo.toml --locked -- --help
cargo run --manifest-path src-tauri/Cargo.toml --locked -- device status
cargo run --manifest-path src-tauri/Cargo.toml --locked -- device status --details
cargo run --manifest-path src-tauri/Cargo.toml --locked -- profiles list
cargo run --manifest-path src-tauri/Cargo.toml --locked -- flash --dry-run
```

`device status` reports serial-port detection and connection state. `--details` includes engineering metadata checks for hardware validation. The desktop UI automatically rescans serial ports every three seconds while it is open, and the **Refresh** button requests an immediate scan. It reports **Connected** only after the Kohlrausch firmware CDC handshake succeeds; a visible USB device without that handshake remains disconnected. The UI omits simulated OLED output and electrical design diagnostics. Firmware flashing is unavailable; `flash --dry-run` does not write firmware.

Run the CLI and firmware-logic checks with `bun test` and `cargo test --manifest-path src-tauri/Cargo.toml --locked`. The interactive Bun terminal prototype is separate from the production Tauri app.

## Profiles and macros

Profiles and macro libraries are validated by the Rust backend and saved in user-writable storage. On Windows, profile storage prefers `%APPDATA%`, then `%LOCALAPPDATA%` or the user profile; on systems without those variables, it uses the user config directory or, as a last resort, the OS temporary directory. Saves use temporary files and backups.

Four layers are stored independently. Macro recording captures key presses only while the application is focused. A saved profile and its referenced macros can be synced to the device from Device settings; the firmware plays back those macro bindings. Website and application actions remain host-only and block sync with a validation error. The fixed visual key arrangement is `3.1 / 2.1 / 1.1`, `3.2 / 2.2 / 1.2`, `3.3 / 2.3 / 1.3`.

## Firmware and hardware status

Firmware is a separate C++ RP2040 target using the Pico SDK and TinyUSB. The current firmware build includes matrix scanning, layer switching, USB keyboard and mouse HID, a CDC identification/status protocol, validated profile/macro sync, and two-slot flash persistence. OLED support is disabled until the controller, resolution, I²C address, bus, and frequency are verified. GPIO assignments (columns GP0–2, rows GP3–5, LYR switch GP10, OLED SDA/SCL GP26/27) and diode direction (columns towards rows) are confirmed; per-key electrical identities remain unverified. See [firmware architecture](docs/firmware-architecture.md) and [device USB protocol](docs/device-protocol.md). Host application builds do not build firmware, and firmware updates are not available through the app.

The Cherry MX switch mesh attribution and license are documented in [model attributions](docs/model-attributions.md).

See [desktop architecture](docs/desktop-architecture.md) and [firmware architecture](docs/firmware-architecture.md) for implementation details.
