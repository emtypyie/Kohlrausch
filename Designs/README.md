# 3x3 Custom Mechanical Macro Pad — Pin Mapping Readme

This document serves as the master hardware reference configuration for the 3x3 mechanical macro pad powered by the ESP32-C3 Super Mini microcontroller module.

## 🎛️ Pinout Reference Table

| ESP32-C3 Pin Name | Schematic Pin # | Project Net Name | Connected Hardware Component | Notes |
| :--- | :--- | :--- | :--- | :--- |
| **5V** | Pin 16 | `VCC_IN` | SPDT Slide Switch Output | Primary power input from battery circuit |
| **GND** | Pin 15 | `GND` | Universal System Ground Link | Common baseline for all components |
| **3V3** | Pin 14 | `3V3_RAIL` | OLED VCC / Potentiometer Pin 3 | Low-voltage power bus line |
| **GPIO0** | Pin 9 | `COL 1` | Keyswitch Matrix Column 1 | Left vertical key lane |
| **GPIO1** | Pin 10 | `COL 2` | Keyswitch Matrix Column 2 | Center vertical key lane |
| **GPIO2** | Pin 11 | `COL 3` | Keyswitch Matrix Column 3 | Right vertical key lane |
| **GPIO3** | Pin 12 | `POT_SIG` | 10K WH148 Volume Potentiometer | Center Wiper Pin (Analog ADC Input) |
| **GPIO4** | Pin 13 | `ROW 1` | Keyswitch Matrix Row 1 | Top horizontal key lane |
| **GPIO5** | Pin 1 | `ROW 2` | Keyswitch Matrix Row 2 | Middle horizontal key lane |
| **GPIO6** | Pin 2 | `ROW 3` | Keyswitch Matrix Row 3 | Bottom horizontal key lane |
| **GPIO7** | Pin 3 | `I2C_SDA` | 0.96" SSD1306 OLED Display | Hardware I2C Serial Data Pin |
| **GPIO10** | Pin 8 | `I2C_SCL` | 0.96" SSD1306 OLED Display | Hardware I2C Serial Clock Pin |
| **GPIO20** | Pin 6 | `DIP1` | 4-Position Switch — Position 1 | Profile Selector Bit 0 |
| **GPIO21** | Pin 7 | `DIP2` | 4-Position Switch — Position 2 | Profile Selector Bit 1 |

---

## 🛑 Strapping Pins (Danger Zone - Keep Isolated)
The following pins monitor boot states at startup and **MUST** remain disconnected:
* **GPIO8 (Pin 5):** Left floating / No Connect.
* **GPIO9 (Pin 4):** Left floating / No Connect. Crucial strapping pin; pulling LOW forces ROM Flashing mode.

---

## ⚡ Power System Layout (TP4056 & Slide Switch)
1. **TP4056 Module:**
   * `IN+` / `IN-`: Unconnected (Uses native onboard Type-C header block).
   * `B+` / `B-`: Solder terminals routed directly to the Lithium Polymer Battery cell leads.
   * `OUT-`: Linked directly to system `GND`.
   * `OUT+`: Wired straight to **Pin 2 (Center Pin)** of the SS12D07 Slide Switch.
2. **SS12D07 Slide Switch:**
   * **Pin 1:** Outputs `VCC_IN` straight to Pin 16 of the ESP32-C3.
   * **Pin 3:** Completely empty. Acts as the isolation cutoff (OFF state).

---

## ⌨️ Matrix Matrix Topology
* **Diodes:** 1N4148 Fast-switching diodes.
* **Direction:** **Column-to-Row** (Current flows OUT of Columns and IN to Rows). 
* **Orientation:** Switch `ROW` pin ➔ Diode Anode (Triangle side) ➔ Diode Cathode (Stripe side) ➔ Horizontal Row Bus.

---

## ⚙️ Binary Profile Switching Truth Table
The 4-Position DIP switch controls system profiles via binary decoding using only two pins:

| DIP Switch 1 (`DIP1`) | DIP Switch 2 (`DIP2`) | Decoded Binary | Target Profile Function |
| :---: | :---: | :---: | :--- |
| OFF (High) | OFF (High) | `00` | **Profile 1:** Standard Keyboard Shortcuts |
| ON (GND) | OFF (High) | `01` | **Profile 2:** Media / Streaming Controls |
| OFF (High) | ON (GND) | `10` | **Profile 3:** Gaming Macro Layout |
| ON (GND) | ON (GND) | `11` | **Profile 4:** Wireless Bluetooth / Alternative Mode |
