# Hardware-free manual scenarios

Run `bun install` once, then launch `node_modules/.bin/kohlrausch.exe` or `node_modules/.bin/khl.exe --c` in a Windows terminal. Quit with `Q`; the terminal alternate screen should be restored.

1. **Fixed keypad and action assignment** — Open Keymaps. Confirm rows read `3.1 / 2.1 / 1.1`, `3.2 / 2.2 / 1.2`, `3.3 / 2.3 / 1.3`. Move focus with arrows, Enter to cycle the selected key's mock action, and Space to activate its mock preview. Switch layers with `1`–`4` and confirm assignments remain layer-specific.
2. **Virtual LYR switch** — Press `L` from any screen. Confirm layer advances 1→2→3→4→1 and navbar status changes. Press a number to select a layer directly.
3. **Macro forms and validation** — Open Macros, use Tab to choose Record, Website, Software or Clicks. Enter opens that mode's local mock form. Type a combo, URL, application label, or `click`/`doubleClick`/`scroll`, then Enter saves. Try an invalid URL or mouse step and confirm validation blocks the save. This never captures or executes OS input.
4. **OLED simulation** — Open OLED and inspect the layer boxes and KOHLRAUSCH screen descriptions, 5-second timing and simulation label. Hardware controller, resolution and address remain pending.
5. **Device and flash boundary** — Device settings must say physical USB disconnected. Press `F`; the flash overlay must say simulation only and confirm no image was sent. Firmware update requests are refused by mock transport.
6. **Configuration and recovery** — Use the schema tests to serialize/parse all four layers. Attempt to write an invalid mock profile; it should reject the write and preserve the previous valid configuration.
