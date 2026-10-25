# Kohlrausch project instructions

These conventions are authoritative for all work in this repository.

## Product and host architecture

- The primary Windows desktop application is native Rust + Slint. Use Clap for CLI parsing and Cargo with a committed `Cargo.lock`.
- The product name shown in the application is `Kohlrausch Konfig`; the `kohlrausch` executable and CLI command stay lowercase.
- Keep the Visrodeck license line and EmtypyieCli package attribution in the About & license view when changing the UI.
- The GUI and CLI share the `kohlrausch.exe` implementation. CLI subcommands must run without starting the GUI. `khl --c` is an alias for opening configuration mode.
- Do not use React, Electron, Tauri, WebView, Node.js, Bun, or OpenTUI for the primary application. `apps/cli` is a retained earlier terminal prototype; do not extend it as the product frontend.
- Do not claim a dependency-free executable. Verify actual release runtime requirements.
- User profiles, configurations, logs, backups, and macros belong in writable user storage such as `%APPDATA%`, never beside the executable.
- Visual editors and Config Studio must converge on one canonical Rust configuration model, schema, validation and persistence path. Config Studio edits actual user files; never maintain a decorative copy.

## UI and safety

- Use a slim navbar, compact sidebar, roomy workspace, restrained status area, and prominent FLASH control. The current visual reference uses near-black surfaces, charcoal panels, warm orange selection accents, and the Visrodeck logo.
- Every visible control must do what its label says. Keep keyboard access and resize behavior. Avoid inert mock buttons and placeholder screens.
- Connection status must report the actual detected hardware state. Never claim a connection without a successful device handshake.
- Flashing remains disabled until identification, target validation, confirmation, progress/error handling and post-flash verification exist. A dry run must never write firmware.
- Do not add global input capture, arbitrary command execution, or host automation without a real implementation and appropriate authorization.

## Hardware and config

- Board metadata is developer-maintained and separate from user profiles. Never expose physical remapping in the ordinary editor.
- Fixed screen layout: `3.1 / 2.1 / 1.1`, `3.2 / 2.2 / 1.2`, `3.3 / 2.3 / 1.3`.
- Keep unknown matrix identities, GPIO, diode behavior, and OLED controller/resolution/address explicitly unverified. Never infer them from labels or illustrations.
- There are four independent layers, startup at 1. Direct editor selection is distinct from a simulated or debounced physical LYR press; LYR cycles 1→2→3→4→1 once per press.
- Validate profiles and macros at all boundaries. Reject unknown or malformed fields; never interpret config as code.
- Use atomic saves and recoverable backups for canonical profile/config writes.

## Firmware

- Firmware is a separate C/C++ RP2040 target using Pico SDK and TinyUSB; host application builds are not firmware builds.
- Keep layer logic independent of GPIO. Abstract GPIO, OLED, USB transport and storage where practical.
- Do not claim physical testing without hardware. Compile against verified hardware metadata only.

## Workflow

- Inspect current files and preserve user work before major changes. This checkout may not contain Git metadata; do not assume a clean baseline.
- Make incremental changes and run relevant formatting, tests, and build checks. Report exact commands and limitations.
- Do not silently drop existing schemas, tests, or features. Ask before destructive migrations, hardware changes, real flashing, or unsafe host automation.
