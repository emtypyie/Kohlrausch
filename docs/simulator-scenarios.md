# CLI and host checks

Hardware-independent simulation and hardware verification scenarios belong in
the CLI and automated host tests. The production Tauri UI does not render a
simulated OLED or expose electrical metadata that has not been verified.

## Application CLI

Run the diagnostic commands from the repository root:

```powershell
cargo run --manifest-path src-tauri/Cargo.toml --locked -- --help
cargo run --manifest-path src-tauri/Cargo.toml --locked -- device status
cargo run --manifest-path src-tauri/Cargo.toml --locked -- device status --details
cargo run --manifest-path src-tauri/Cargo.toml --locked -- profiles list
cargo run --manifest-path src-tauri/Cargo.toml --locked -- flash --dry-run
```

`device status` lists detected serial ports and probes USB CDC devices with the
Kohlrausch protocol handshake. A port counts as identified only when the board
ID and firmware version reply match. `device status --details` also reports
board facts still pending verification. `flash --dry-run` never writes
firmware; normal flashing remains disabled.

## Automated host checks

Run the retained TypeScript simulator tests and Tauri Rust tests:

```powershell
bun test
cargo test --manifest-path src-tauri/Cargo.toml --locked
```

The Bun suite covers the test-only mock transport, simulated layer and OLED
logic, schema validation, and terminal prototype components. These tests do not
claim physical device behavior.

The portable firmware logic has separate C++ host tests:

```powershell
cmake -S firmware/tests -B target/firmware-host-build
cmake --build target/firmware-host-build --config Debug
ctest --test-dir target/firmware-host-build -C Debug --output-on-failure
```

This validates hardware-independent firmware logic only. It does not build an
RP2040 image or verify physical wiring, OLED parts, or device behavior.
