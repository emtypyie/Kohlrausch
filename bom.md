# Bill of Materials (BOM) - Project Kohlrausch (Single Unit)

This document lists the exact components required to construct **one (1) individual Unit** 

---

## 1. Core Electronics & Interface

| Component Name | Description | Required Qty | Sourcing / Build Notes |
| :--- | :--- | :---: | :--- |
| **RP2040** | Native USB Support | **1 pc** | Small footprint and easy to use with micropython |
| **3-Pin Mechanical Switches** | MX-style mechanical switches (e.g., Gateron/Kailh Red, Brown, or Blue). | **10 pcs** | Standard 3-pin switches are perfect; the 3D-printed plate handles stabilization. |
| **Kailh PCB Hot-swap Sockets** | Leaf-spring sockets soldered onto the PCB to swap switches without tools. | **10 pcs** | Buy standard MX-compatible sockets (avoid Choc/low-profile variants). |
| **0.92" I2C OLED Display** | 128x64 pixel resolution, standard 4-pin monochrome screen (SSD1306 driver). | **1 pc** | Double-check pin order on the breakout board when soldering (GND/VCC orientation). |



---


## 4. Materials & Shared Tools Checklist

*   **3D Printer Filament (PLA or PETG):** Approximately 75 grams total for one shell case and switch plate.
*   **Solid Core Hookup Wire (22-24 AWG):** 1 small spool. Used for physical breadboard layout connections and wiring matrix layers.
*   **Custom Manufactured PCB:** 1 batch order (standard minimum order is 5 boards from fabricators like JLCPCB). You will get 5 boards, using 1 for yourself and keeping the rest as spares.
*   **Consumables:** Soldering iron, solder wire, soldering flux pen (critical for attaching wide hotswap tabs cleanly), and Isopropyl Alcohol (99%) to clean off sticky resin residue.
