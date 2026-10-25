# Kohlrausch Konfig

## Requirements

Install the Rust stable toolchain and the Windows C++ build tools required by Slint. Run the following commands from the project directory.

## Build and launch

~~~powershell
cargo build --locked
cargo run --locked
cargo run --locked -- --config
~~~

The default window opens in Home. Use the left navigation or click the model. Drag the model to rotate it, scroll to zoom, and choose a layer below the model. `Esc` closes the window.

The repository-local launch commands do not install anything globally:

~~~powershell
.\tools\kohlrausch.cmd
.\tools\khl.cmd --c
~~~

`kohlrausch` opens Home. `khl --c` opens Config Studio.

## Other commands

~~~powershell
cargo run --locked -- --help
cargo run --locked -- device status
cargo run --locked -- profiles list
cargo run --locked -- flash --dry-run
cargo test --locked
~~~

`device status` reports whether a compatible device has been verified. Selecting a serial port alone does not connect to a device. `flash --dry-run` does not write firmware.

## Keymap and macros

Key assignments are saved to the active local profile. Four layers are stored independently. Macro entries are validated and previewed in the application; host automation is not enabled in this release.

The case, PCB and USB geometry come from the specified FreeCAD case file. Cherry MX switch geometry is attributed in [model attributions](docs/model-attributions.md). The board's electrical switch mapping and OLED specifications remain pending schematic verification.
