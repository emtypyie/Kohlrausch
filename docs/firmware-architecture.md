# Firmware architecture

This document describes the separate RP2040 firmware target. It is not part of
the Tauri desktop application's host build.

The current desktop application does not simulate physical device or OLED
behavior. Hardware-independent logic and mock transports are exercised by CLI
and host-side tests only; see [CLI and host checks](simulator-scenarios.md).

The firmware target is a separate C++ project for the RP2040 and Raspberry Pi
Pico SDK. This repository currently contains hardware-independent runtime logic,
port interfaces, and host-side tests. It does not yet contain a board scanner,
OLED driver, USB implementation, persistent-storage driver, or a physically
validated firmware image.

## Implemented logic

- `LayerManager` starts on layer 1 and advances through layers 1–4.
- `DebouncedPress` reports one event for each stable press transition. It uses
  unsigned elapsed-time arithmetic, so the timer can wrap safely. The runtime
  default is 20 ms; the hardware driver must provide a normalized semantic
  pressed state rather than assumed GPIO polarity.
- `OledCycle` produces a layer-selector frame for five seconds followed by a
  branding frame for five seconds. It catches up after delayed polls without
  sleeping. A layer change immediately selects the new layer frame and restarts
  the interval.
- `FirmwareRuntime` combines LYR debouncing, layer state, and display scheduling.
  The eventual main loop can render only when `oled_frame_changed` is true.
- `firmware/include/hardware_ports.hpp` separates semantic LYR input, OLED
  rendering, USB HID, USB configuration, and keymap storage interfaces.

The runtime logic performs no GPIO reads, display I/O, USB traffic, file access,
or dynamic allocation. Implementations of the interfaces must remain outside the
logic layer.

## Hardware facts still pending

The fixed visual key arrangement is recorded in the board definition. The
electrical matrix row/column identity for each switch, row and column GPIOs,
diode direction, LYR GPIO and polarity, and OLED controller, resolution, and
I2C address remain unverified against the schematic. No scanner, display driver,
or USB device identity is guessed here. The RP2040 entry point remains
intentionally inactive until the verified board drivers are available.

## Host-side verification

Build and run the hardware-independent C++ tests on Windows:

```powershell
cmake -S firmware/tests -B target/firmware-host-build
cmake --build target/firmware-host-build --config Debug
ctest --test-dir target/firmware-host-build -C Debug --output-on-failure
```

This proves the portable logic compiles and its unit tests pass. It is not an
RP2040 firmware build and does not verify any hardware behavior.

## Pico SDK build

Configure the real firmware target only after the Pico SDK and ARM cross-toolchain
are installed and available in the environment:

```powershell
$env:PICO_SDK_PATH = "C:\path\to\pico-sdk"
cmake -S firmware -B target/firmware-pico-build -G Ninja
cmake --build target/firmware-pico-build
```

The Pico SDK, CMake generator, and ARM cross-toolchain must be installed and
configured for the selected environment before this command can produce an
RP2040 image. A host build is not evidence that firmware compiled for or ran on
the macropad.
