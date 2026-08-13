# Kohlrausch Macro Pad

A compact USB macro pad based on the **RP2040 Zero**, featuring a 3×3 hot-swappable mechanical key matrix, OLED status display, and a dedicated layer-switch key.

---

## 1. Hardware Overview

### Main Components

* **RP2040 Zero** — main controller
* **9× hot-swappable mechanical switches**
* **9× 1N4148 switching diodes**
* **1× dedicated mechanical layer switch**
* **0.96" I²C OLED**
* **4× 3.2 mm NPTH mounting holes**
* **M3 × 15 mm screws**
* **M3 brass heat-set inserts** for the enclosure

The potentiometer was removed from the current revision.

---

## 2. System Architecture

```text
                    ┌──────────────────────┐
                    │      RP2040 Zero     │
                    │                      │
3×3 Key Matrix ────►│ GP0–GP5              │
                    │                      │
Layer Switch ──────►│ GP10                 │
                    │                      │
OLED ──────────────►│ GP8 / GP9            │
                    │                      │
USB ◄──────────────►│ USB HID              │
                    └──────────────────────┘
```

The RP2040 handles:

* Matrix scanning
* Layer switching
* OLED communication
* USB HID keyboard output
* Macro execution
* Layer/profile management

---

# 3. GPIO Assignment

| RP2040 GPIO | Function     | Type          |
| ----------- | ------------ | ------------- |
| **GP0**     | COL1         | Matrix        |
| **GP1**     | COL2         | Matrix        |
| **GP2**     | COL3         | Matrix        |
| **GP3**     | ROW1         | Matrix        |
| **GP4**     | ROW2         | Matrix        |
| **GP5**     | ROW3         | Matrix        |
| GP6         | NC           | Reserved      |
| GP7         | NC           | Reserved      |
| **GP8**     | OLED SDA     | I²C           |
| **GP9**     | OLED SCL     | I²C           |
| **GP10**    | Layer Switch | Digital Input |
| GP11        | NC           | Reserved      |
| GP12        | NC           | Reserved      |
| GP13        | NC           | Reserved      |
| GP14        | NC           | Reserved      |
| GP15        | NC           | Reserved      |
| GP26        | NC           | Reserved ADC  |
| GP27        | NC           | Reserved ADC  |
| GP28        | NC           | Reserved ADC  |
| GP29        | NC           | Reserved ADC  |

Unused GPIOs are intentionally left unconnected and marked **NC (No Connect)** in the schematic.

---

# 4. 3×3 Key Matrix

The keyboard consists of a 3×3 diode-isolated matrix.

```text
             COL1        COL2        COL3

ROW1         1.1         2.1         3.1

ROW2         1.2         2.2         3.2

ROW3         1.3         2.3         3.3
```

### Matrix Connections

```text
COL1 → GP0
COL2 → GP1
COL3 → GP2

ROW1 → GP3
ROW2 → GP4
ROW3 → GP5
```

Each key follows:

```text
COL ── SWITCH ──|>|── ROW
                 │
               1N4148
```

The diodes provide electrical isolation between keys and prevent common matrix ghosting conditions.

---

# 5. Switch Placement

The key matrix uses a **750 mil pitch**.

```text
750 mil = 19.05 mm
```

### Exact Switch Coordinates

| Key | X (mil) | Y (mil) |
| --- | ------: | ------: |
| 1.1 |     970 |    -480 |
| 2.1 |    1720 |    -480 |
| 3.1 |    2470 |    -480 |
| 1.2 |     970 |     270 |
| 2.2 |    1720 |     270 |
| 3.2 |    2470 |     270 |
| 1.3 |     970 |    1020 |
| 2.3 |    1720 |    1020 |
| 3.3 |    2470 |    1020 |

### Pitch

```text
Horizontal pitch = 750 mil = 19.05 mm
Vertical pitch   = 750 mil = 19.05 mm
```

The current switch grid therefore spans:

```text
X = 970 → 2470 mil
Y = -480 → 1020 mil
```

The final external PCB dimensions will be determined after the complete component placement and enclosure design are finalized.

---

# 6. Matrix Diodes

There are **9 × 1N4148** diodes.

The diodes are intended to be mounted on the **bottom side of the PCB**, close to their corresponding hot-swap sockets.

Recommended arrangement:

```text
Switch 1.1 → D1
Switch 2.1 → D4
Switch 3.1 → D7

Switch 1.2 → D2
Switch 2.2 → D5
Switch 3.2 → D8

Switch 1.3 → D3
Switch 2.3 → D6
Switch 3.3 → D9
```

The physical diode orientation must remain electrically identical to the schematic:

```text
COL → SWITCH → DIODE → ROW
```

Moving a diode to the bottom layer does not change its electrical orientation.

---

# 7. Layer Switch

The additional mechanical switch is **not part of the 3×3 matrix**.

It is an independent GPIO input.

### Connection

```text
GP10 ── LAYER SWITCH ── GND
```

Firmware should configure GP10 using the internal pull-up:

```cpp
pinMode(10, INPUT_PULLUP);
```

Result:

```text
Switch released → HIGH
Switch pressed  → LOW
```

The switch can therefore be used to cycle through macro layers.

A spare diode is currently retained on this standalone switch because spare 1N4148 diodes are available. It is not required for matrix ghosting protection.

---

# 8. OLED Display

The design uses a **0.96" 4-pin I²C OLED module**.

### OLED Connections

| OLED Pin | RP2040  |
| -------- | ------- |
| SDA      | **GP8** |
| SCL      | **GP9** |
| VCC      | **3V3** |
| GND      | **GND** |

The OLED should be powered from **3.3 V**.

Check the exact OLED module for its onboard I²C pull-up resistors before adding external pull-ups.

---

# 9. Power

### 3.3 V

```text
RP2040 3V3
    │
    └── OLED VCC
```

### Ground

```text
RP2040 GND
    ├── OLED GND
    ├── Layer Switch GND
    └── Common circuit GND
```

The RP2040 Zero module handles its own onboard regulation and USB power circuitry.

Do not connect the OLED VCC directly to 5 V unless the exact OLED module is confirmed to support 5 V operation.

---

# 10. USB

USB is handled by the **RP2040 Zero's onboard USB interface**.

The macro pad will operate as a USB HID device.

The carrier PCB does not need to route the RP2040 USB D+ and D− signals separately when using the complete RP2040 Zero module.

Expected host behavior:

```text
Macro Pad
    │
    │ USB
    ▼
Computer
    │
    └── USB HID Keyboard
```

The firmware can therefore send normal keyboard HID reports and macro sequences.

---

# 11. Mounting Holes

Four **3.2 mm NPTH mounting holes** are positioned between the switches.

They are intended for:

* M3 × 15 mm screws
* M3 brass heat-set inserts in the enclosure

### Exact Mounting Hole Coordinates

| Hole   |  X (mil) |  Y (mil) |
| ------ | -------: | -------: |
| **H1** | **1345** | **-105** |
| **H2** | **2095** | **-105** |
| **H3** | **1345** |  **645** |
| **H4** | **2095** |  **645** |

These coordinates place each hole exactly in the center of the gap between four neighboring switches.

Calculation:

```text
970 + 375 = 1345
1720 + 375 = 2095

-480 + 375 = -105
270 + 375 = 645
```

### Hole Specification

```text
Hole type:       NPTH
Hole diameter:   3.2 mm
Screw:            M3 × 15 mm
Case insert:      M3 brass heat-set insert
```

The heat-set inserts are installed into the **3D-printed case**, not the PCB.

---

# 12. PCB Component Placement

## Top Side

The intended top-side components are:

```text
9 × hot-swap switch sockets
1 × layer switch
1 × OLED
```

The top side is intended to remain user-facing and visually clean.

## Bottom Side

The intended bottom-side components are:

```text
9 × 1N4148 diodes
RP2040 Zero
```

The RP2040 Zero can be mounted underneath the PCB provided that:

* The USB connector remains accessible.
* The enclosure has sufficient clearance.
* The RP2040 Zero does not collide with the mounting hardware.
* The module's component height is accounted for.
* The PCB does not interfere with the case.

---

# 13. Macro Layer System

The physical matrix contains:

```text
3 × 3 = 9 physical keys
```

With four firmware layers:

```text
Layer 0 → 9 macros
Layer 1 → 9 macros
Layer 2 → 9 macros
Layer 3 → 9 macros
```

Total:

```text
4 × 9 = 36 macro assignments
```

Additional layers can be implemented entirely in firmware without modifying the PCB.

Example:

```text
Layer 0 → General
Layer 1 → Development
Layer 2 → Browser
Layer 3 → Media
```

The OLED can display the currently active layer.

Example:

```text
┌───────────────┐
│    LAYER 02   │
│               │
│   MACRO PAD   │
└───────────────┘
```

---

# 14. Firmware Matrix Model

The firmware should scan the matrix using:

```text
COL1 → GP0
COL2 → GP1
COL3 → GP2

ROW1 ← GP3
ROW2 ← GP4
ROW3 ← GP5
```

The layer switch is handled independently:

```text
LAYER SWITCH → GP10
```

The OLED is handled through I²C:

```text
SDA → GP8
SCL → GP9
```

---

# 15. PCB Design Checklist

Before manufacturing, verify:

* [ ] 9 hot-swap switch footprints are present.
* [ ] All switches use the intended footprint.
* [ ] Switch centers match 750 mil / 19.05 mm pitch.
* [ ] All 9 matrix diodes are present.
* [ ] All diode orientations match the schematic.
* [ ] Diodes are placed close to their corresponding sockets.
* [ ] Matrix columns are connected to GP0–GP2.
* [ ] Matrix rows are connected to GP3–GP5.
* [ ] OLED SDA is connected to GP8.
* [ ] OLED SCL is connected to GP9.
* [ ] Layer switch is connected to GP10.
* [ ] Unused GPIOs are marked NC.
* [ ] OLED VCC is connected to 3V3.
* [ ] OLED GND is connected to GND.
* [ ] Layer switch has a GND return.
* [ ] Four 3.2 mm NPTH mounting holes are present.
* [ ] Mounting holes use the specified coordinates.
* [ ] Mounting holes do not interfere with switch/socket footprints.
* [ ] RP2040 Zero has sufficient case clearance.
* [ ] USB connector has a case opening.
* [ ] Bottom-side diode clearance is verified.
* [ ] PCB outline is finalized.
* [ ] Schematic ERC is clean.
* [ ] PCB DRC is clean.
* [ ] 3D enclosure clearance is verified.

---

# 16. Current Design Status

**Revision:** PCB placement / routing preparation.

### Completed

* RP2040 Zero selected
* GPIO allocation established
* 3×3 matrix established
* 9 matrix diodes established
* OLED interface established
* Dedicated layer switch established
* Potentiometer removed
* 750 mil / 19.05 mm key pitch established
* Internal M3 mounting-hole coordinates established
* Top/bottom component strategy established

### Remaining

1. Finalize diode placement.
2. Finalize OLED placement.
3. Finalize RP2040 Zero placement.
4. Finalize layer-switch placement.
5. Finalize PCB outline.
6. Route matrix traces.
7. Route OLED and layer-switch connections.
8. Complete power and ground routing.
9. Run schematic ERC.
10. Run PCB DRC.
11. Verify enclosure clearance.
12. Perform final manufacturing review.

---

# 17. Quick Reference

```text
CONTROLLER
RP2040 Zero

MATRIX
3 × 3 keys
750 mil / 19.05 mm pitch

COLUMNS
COL1 → GP0
COL2 → GP1
COL3 → GP2

ROWS
ROW1 → GP3
ROW2 → GP4
ROW3 → GP5

OLED
SDA → GP8
SCL → GP9
VCC → 3V3
GND → GND

LAYER SWITCH
GP10 → Switch → GND

DIODES
D1–D9 → 1N4148

MOUNTING HOLES
H1 → 1345, -105 mil
H2 → 2095, -105 mil
H3 → 1345,  645 mil
H4 → 2095,  645 mil

MOUNTING HOLE DIAMETER
3.2 mm NPTH

KEY PITCH
750 mil = 19.05 mm

MACRO CAPACITY
9 keys × 4 layers = 36 macros
```

---

## Design Principle

The hardware is intentionally kept simple:

**RP2040 Zero = controller and USB HID**

**3×3 matrix = physical input**

**1 dedicated switch = layer control**

**OLED = user feedback**

**Firmware = macro and layer logic**

This keeps the PCB compact while leaving the majority of the device's functionality configurable in software.
