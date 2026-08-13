# Kohlrausch Macro Pad

A compact USB macro pad based on the **RP2040 Zero**, featuring a 3×3 hot-swappable mechanical key matrix, OLED status display, and a dedicated layer-switch key.

---

## 1. Hardware Overview

### Main Components

- **RP2040 Zero** — main controller
- **9× hot-swappable mechanical switches**
- **9× 1N4148 switching diodes**
- **1× dedicated mechanical layer switch**
- **0.91" 128×32 SSD1306 I²C OLED**
- **4× 3.2 mm NPTH mounting holes**
- **M3 × 15 mm screws**
- **M3 brass heat-set inserts** for the enclosure

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
OLED ──────────────►│ GP26 / GP27          │
                    │                      │
USB ◄──────────────►│ USB HID              │
                    └──────────────────────┘
