# Firmware architecture

The RP2040 firmware is a separate C++ target built with the Raspberry Pi Pico
SDK and TinyUSB. It is independent of the Tauri desktop build. Firmware source
uses the legacy project notes in `decades/Frimware` for the design pin map;
the board definition still marks electrical details as pending physical
verification.

## Implemented firmware

- Composite USB device with CDC configuration/identification and HID keyboard
  and mouse report IDs.
- Versioned CDC handshake: `KOHLRAUSCH/1 HELLO` identifies the board and
  firmware version; `KOHLRAUSCH/1 STATUS` reports the active layer.
- Bounded configuration protocol with CRC validation and two-slot flash
  persistence. The newest valid image is loaded at boot; the previous image is
  retained for recovery.
- Keyboard chord, mouse action, and macro bindings. Macros support recorded or
  fixed timing and once, until-stop, until-any-click, or counted looping.
- 3×3 matrix scan on columns GP0–2 and rows GP3–5, with column-to-row diode
  scanning (diodes oriented from column towards row) and 20 ms key debounce.
- LYR switch on GP10, active-low with 300 ms debounce, cycling layers
  1→2→3→4→1.
- OLED display on I²C GP26/27 (assumed SSD1306 128×32 at 0x3C, 100 kHz — pending
  physical verification). It displays the active layer and cycles to a Kohlrausch
  wordmark every five seconds.
- Empty fallback configuration. Layers and macros come from a validated
  desktop sync; the old archived A–I mapping is not compiled in as a hidden
  default.

Website and application actions are host-only. The desktop sync builder
rejects them with an error instead of silently omitting them. The firmware
does not execute host-side actions, and it does not flash itself.

## Hardware facts and verification

GPIO assignments (columns GP0–2, rows GP3–5, LYR switch GP10, and OLED SDA/SCL
GP26/27) and diode direction (columns towards rows) are confirmed from design
schematics. OLED controller model, OLED resolution, OLED I²C address, and bus
speed remain unconfirmed pending physical verification on the actual hardware.
Keep `status: "pending-verification"` for the OLED section in the board
definition until the physical display is checked. The current firmware build
disables OLED support. Enabling it requires verified controller, resolution,
address, bus, and frequency values; CMake now fails closed when those values
are unknown.

The firmware was built and installed to the connected RP2040 in BOOTSEL mode.
Windows enumerated its CDC and HID interfaces, and the Kohlrausch CDC
identification handshake succeeded on COM3. Key mapping, rollover, LYR
switch behavior, macro playback, flash persistence, and OLED behavior have not
been physically validated. OLED support is disabled in this build. The
configured USB PID is a development placeholder and must be replaced with an
assigned production PID before distributing hardware.

## Build and tests

Install the Pico SDK, initialize its TinyUSB submodule, and install the ARM
cross-toolchain before configuring:

```powershell
$env:PICO_SDK_PATH = (Resolve-Path 'pico-sdk').Path
cmake -S firmware -B work/firmware-pico-build-ninja -G Ninja -DCMAKE_BUILD_TYPE=Release -DPICO_BOARD=pico
cmake --build work/firmware-pico-build-ninja --parallel
```

The expected UF2 is emitted under `work/firmware-pico-build-ninja`. Host
builds of the Tauri application do not build firmware.

Run the hardware-independent firmware tests separately:

```powershell
cmake -S firmware/tests -B work/firmware-tests -G Ninja
cmake --build work/firmware-tests --parallel
ctest --test-dir work/firmware-tests --output-on-failure
```

A successful host C++ logic build does not verify the RP2040 image or hardware
behavior.
