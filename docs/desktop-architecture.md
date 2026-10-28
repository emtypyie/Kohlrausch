# Desktop application architecture

Kohlrausch Konfig's current desktop application is a Tauri 2 application with a Rust host and a bundled static web UI. A standalone splash window shows the bundled banner and a five-second progress bar. The main window stays hidden until the minimum splash interval has elapsed and initial workspace loading completes. The earlier Slint implementation is retained in `slint-build/`; `apps/cli` is a retained terminal prototype.

## Components

- `src-tauri/src/main.rs` dispatches CLI subcommands before starting the desktop window.
- `src-tauri/src/lib.rs` constructs the Tauri app, registers commands, and selects the startup section.
- `src-tauri/ui/index.html`, `main.js`, `styles.css`, and `scene.js` implement the desktop UI and 3D model view. Tauri serves the UI from `src-tauri/ui` as configured in `tauri.conf.json`.
- `src-tauri/ui/splash.html`, `splash.css`, and `splash.js` render the standalone startup window using the bundled banner asset.
- `src-tauri/src/commands.rs` exposes the UI's profile, key assignment, configuration import/save, macro, and serial-port operations.
- `src-tauri/src/config.rs` owns the Rust profile and macro types, validation, migration, and persistence. UI input is data passed to these commands; it is validated by Rust before writes.
- `src-tauri/build.rs` derives a 16-character hexadecimal Build ID from the build inputs and Rust target. The About page reports that fingerprint alongside the package version; the Build ID is not a Git commit hash.

## Data and hardware boundaries

Profile files, layer files, the active-profile marker, and the macro library are stored in the user's writable configuration area. On Windows, storage prefers `%APPDATA%`; when those environment variables are absent, it falls back through other user paths and finally the OS temporary directory. Individual profile file writes use temporary files, backups, and atomic replacement.

The desktop app enumerates serial ports and reports its connection state. It does not show a simulated OLED or expose electrical design diagnostics. Firmware flashing is disabled. Board matrix identities, GPIO mapping, diode behavior, and OLED specifications remain pending engineering verification in the board metadata and CLI diagnostics. The RP2040 firmware project is independent of the host application build; see [firmware architecture](firmware-architecture.md).

## Build and CLI

Run Cargo with the Tauri manifest from the repository root:

```powershell
cargo build --manifest-path src-tauri/Cargo.toml --locked
cargo run --manifest-path src-tauri/Cargo.toml --locked
cargo run --manifest-path src-tauri/Cargo.toml --locked -- --config
```

CLI subcommands such as `device status`, `device status --details`, `profiles list`, and `flash --dry-run` exit before Tauri starts. Detailed hardware checks are CLI-only. The dry run does not write firmware. The root Bun `start`, `config`, and `kohlrausch` scripts invoke this Tauri application; `bun run prototype` launches the retained terminal prototype.

## Release builds

The GitHub Actions workflow builds Windows MSI and NSIS installers, Windows portable EXE and ZIP packages, and Linux DEB and AppImage packages. It reads the package version from `src-tauri/Cargo.toml` and rejects a mismatch with `tauri.conf.json` or a pushed tag that differs from `v<version>`. See [release build details](releasing.md).
