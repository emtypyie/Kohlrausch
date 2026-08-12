# Bill of Materials (BOM) - Project Kohlrausch (Single Unit)

This document lists the exact components required to construct **one (1) individual Unit** 

---

## 1. Core Electronics & Interface

| Component Name | Description | Required Qty | Sourcing / Build Notes |
| :--- | :--- | :---: | :--- |
| **ESP32-C3 Super Mini Board** | HW-466AB form factor. Handles Native USB-HID & Bluetooth LE. | **1 pc** | Ensure it is the "Super Mini" variant to fit the compact case layout. |
| **3-Pin Mechanical Switches** | MX-style mechanical switches (e.g., Gateron/Kailh Red, Brown, or Blue). | **9 pcs** | Standard 3-pin switches are perfect; the 3D-printed plate handles stabilization. |
| **Kailh PCB Hot-swap Sockets** | Leaf-spring sockets soldered onto the PCB to swap switches without tools. | **9 pcs** | Buy standard MX-compatible sockets (avoid Choc/low-profile variants). |
| **0.96" I2C OLED Display** | 128x64 pixel resolution, standard 4-pin monochrome screen (SSD1306 driver). | **1 pc** | Double-check pin order on the breakout board when soldering (GND/VCC orientation). |
| **10kΩ Linear Potentiometer** | Panel-mount rotary volume dial / analog macro scroller. | **1 pc** | Choose a smooth, non-detent shaft profile so any standard knob fits perfectly. |
| **3mm or 5mm Standard LED** | Simple low-current indicator bulb for immediate keypress status flashes. | **1 pc** | Pick your favorite color to personalize your status flash indicator. |
| **SPDT Mini Slide Switch** | Three-pin physical slide toggle used for the Profile Selection feature. | **1 pc** | Sub-miniature layout to cleanly sit along the side edge of your casing walls. |

---

## 2. Power & Battery Hardware

| Component Name | Description | Required Qty | Sourcing / Build Notes |
| :--- | :--- | :---: | :--- |
| **Type-C Battery Charging Module** | External TP4056 linear charging protection board. | **1 pc** | Handles safe battery replenishment and output cutoff protection. |
| **3.7V LiPo Battery Cell** | Compact rechargeable flat cell (500mAh to 1000mAh capacity recommended). | **1 pc** | Measure your internal 3D-printed case depth before ordering cell thickness. |
| **SPDT Power Slide Switch** | Heavy-duty physical switch to cut battery ground line completely. | **1 pc** | Essential to prevent residual battery drain when the keypad is not in use. |

---

## 3. Passive Components & Small Hardware Packs

| Component Name | Description | Needed Qty | Minimum Order Pack Size | Sourcing / Build Notes |
| :--- | :--- | :---: | :---: | :--- |
| **1N4148 Switching Diodes** | Fast axial diodes used to prevent matrix ghosting pathways. | 9 pcs | **1 Pack (50 pcs)** | Solder one diode per hot-swap socket tab. |
| **220Ω - 330Ω Resistors** | 1/4 Watt metal film current-limiting resistor for the status LED. | 1 pc | **1 Pack (10-20 pcs)** | Wired in-line with the status LED positive input leg to prevent burnout. |
| **M3 Countersunk (CSK) Screws** | 15mm overall length flat-top beveled casing assembly hardware. | 4 pcs | **1 Pack (10 pcs)** | Requires modeling an angled countersunk chamfer bevel in your 3D case holes. |
| **M3 Brass Heat-Set Inserts** | Threaded cylinders melted into 3D-printed pillars with a soldering iron. | 4 pcs | **1 Pack (10 pcs)** | Allows you to open and modify the case without stripping plastic. |
| **Standard 1U Keycaps** | MX-stem cross compatible plastic keycap covers. | 9 pcs | **1 Set (10+ pcs)** | Translucent or clear keycaps work best if you add RGB lighting paths later. |

---

## 4. Materials & Shared Tools Checklist

*   **3D Printer Filament (PLA or PETG):** Approximately 75 grams total for one shell case and switch plate.
*   **Solid Core Hookup Wire (22-24 AWG):** 1 small spool. Used for physical breadboard layout connections and wiring matrix layers.
*   **Custom Manufactured PCB:** 1 batch order (standard minimum order is 5 boards from fabricators like JLCPCB). You will get 5 boards, using 1 for yourself and keeping the rest as spares.
*   **Consumables:** Soldering iron, solder wire, soldering flux pen (critical for attaching wide hotswap tabs cleanly), and Isopropyl Alcohol (99%) to clean off sticky resin residue.
