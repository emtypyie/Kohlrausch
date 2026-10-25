# Firmware architecture (Milestone 1)

Firmware is not operational. Future implementation targets Raspberry Pi Pico SDK and TinyUSB. GPIO scanning, OLED rendering, USB CDC/HID and persistent storage must sit behind narrow interfaces. Layer cycling and validation logic remain host-testable, independent of GPIO and display drivers.

Unknown details remain null until verified against the schematic: matrix row/column identities, GPIO assignments, diode direction, LYR GPIO, OLED controller/resolution/address. No guessed display driver or matrix scanner is included. The simulator refuses firmware update requests.


## Build boundary

Host-testable logic builds with cmake -S firmware/tests -B work/firmware-host-build, cmake --build work/firmware-host-build, and ctest --test-dir work/firmware-host-build -C Debug --output-on-failure. The RP2040 CMake project requires a verified Pico SDK checkout in PICO_SDK_PATH; configuration fails clearly when that variable is absent. No physical board or final GPIO/OLED/USB drivers are available in this milestone.

