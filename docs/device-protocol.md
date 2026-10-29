# Device USB protocol

The RP2040 firmware presents a composite USB device with a CDC serial
interface and a HID interface containing keyboard and mouse report IDs.
Opening an arbitrary serial port or seeing the RP2040 bootloader is not enough
to identify a Kohlrausch device; the versioned handshake must reply.

## Identification

The host sends one UTF-8 line:

```text
KOHLRAUSCH/1 HELLO\n
```

Firmware responds with:

```text
KOHLRAUSCH/1 DEVICE kohlrausch-rp2040-macropad <firmware-version>\r\n
```

The status command is:

```text
KOHLRAUSCH/1 STATUS\n
```

Its response is `KOHLRAUSCH/1 STATUS layer=<1-4>`. Unsupported commands return
`KOHLRAUSCH/1 ERROR unsupported-command`. The firmware uses a bounded
256-byte command buffer and rejects overlong lines.

## HID reports

- Keyboard: report ID `1`, standard six-key boot-style report.
- Mouse: report ID `2`, five-button mouse with vertical wheel.
- The HID interface uses protocol mode `none` because it carries multiple
  report IDs.

## Configuration transfer

The host sends newline-terminated text commands. Binary configuration bytes
are transported as uppercase hexadecimal. The maximum serialized
configuration is 8192 bytes and the host sends it in 64-byte chunks.

```text
KOHLRAUSCH/1 CONFIG BEGIN <size> <crc32>\n
KOHLRAUSCH/1 CONFIG DATA <offset> <hex>\n
KOHLRAUSCH/1 CONFIG COMMIT <size> <crc32>\n
KOHLRAUSCH/1 CONFIG STATUS\n
KOHLRAUSCH/1 CONFIG RESET\n
```

Successful responses:

```text
KOHLRAUSCH/1 CONFIG READY <size>\r\n
KOHLRAUSCH/1 CONFIG OK <end-offset>\r\n
KOHLRAUSCH/1 CONFIG SAVED <crc32>\r\n
KOHLRAUSCH/1 CONFIG STATUS source=flash|default version=1 sequence=<n> bytes=<n> crc=<crc32>\r\n
KOHLRAUSCH/1 CONFIG RESET\r\n
```

Failures use `KOHLRAUSCH/1 CONFIG ERROR <code>`. The firmware rejects
out-of-order chunks, oversized or truncated payloads, invalid CRCs, unknown
schema versions, malformed bindings, invalid macro references, and reserved
fields that are not zero. `CONFIG RESET` writes the empty fallback
configuration.

`crc32` is the standard IEEE CRC-32 with polynomial `0xEDB88320`.

## Configuration payload

All integers are little-endian.

- Header: magic `0x3146434B` (`KCF1`), schema `1`, four layers, macro count,
  total byte length, reserved zero.
- Layers: four layers in layer order, each with nine eight-byte bindings in
  the fixed visual key order `3.1, 2.1, 1.1, 3.2, 2.2, 1.2, 3.3, 2.3, 1.3`.
- Binding: kind, modifiers, keycode, macro index, mouse action, three reserved
  bytes. Kinds are `0` empty, `1` keyboard chord, `2` mouse action, `3` macro
  reference.
- Macros: a twelve-byte header followed by up to 64 twelve-byte steps. The
  header carries step count, loop mode, loop count, fixed delay, timing mode,
  and reserved bytes. Each step carries delay, kind, modifiers, keycode, mouse
  action, and reserved bytes.

The desktop builder validates the canonical Rust profile and macro library
before constructing this payload. It rejects profile or macro references that
are missing, more than eight referenced macros, and host-only actions.

## Persistence and recovery

The firmware stores the last two configuration images in two 8 KB flash slots
at the end of the RP2040 flash. Each slot has a magic value, schema, sequence
number, payload size, and CRC. On boot the firmware validates both slots and
uses the newest valid image; if neither is valid it falls back to the empty
configuration. Writes erase and program one slot through the Pico SDK flash
API, preserving the previous slot for recovery.

## Unsupported actions and flashing

Website and application actions cannot run on the macropad. The desktop sync
builder rejects them instead of silently dropping them, and macros containing
those steps are rejected as well. Firmware flashing is not implemented; the
desktop flash control remains disabled and `flash --dry-run` does not write
firmware. No physical hardware validation is claimed by this protocol.
