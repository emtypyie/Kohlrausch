#pragma once

#include <cstdint>

#include "../src/runtime_logic.hpp"

namespace kohlrausch::ports {

// Stable key IDs follow the fixed component-side visual layout. They do not
// encode matrix coordinates or GPIO pins; the electrical mapping is pending.
enum class KeyId : std::uint8_t {
  Key3_1,
  Key2_1,
  Key1_1,
  Key3_2,
  Key2_2,
  Key1_2,
  Key3_3,
  Key2_3,
  Key1_3,
};

struct KeyEvent {
  KeyId key;
  bool pressed;
};

// Board-specific implementations are intentionally absent until the schematic
// verifies GPIO assignments, switch polarity, OLED controller and I2C address.
class KeyScanner {
 public:
  virtual ~KeyScanner() = default;
  virtual void poll(std::uint32_t now_ms) = 0;
  virtual bool next_event(KeyEvent& event) = 0;
};

class LyrInput {
 public:
  virtual ~LyrInput() = default;
  virtual bool is_pressed() = 0;
};

class OledOutput {
 public:
  virtual ~OledOutput() = default;
  virtual void render(const OledFrame& frame) = 0;
};

// USB HID, USB CDC/configuration, and persistent profile storage are separate
// ports so transport or storage code cannot leak into the layer state machine.
class UsbHidOutput {
 public:
  virtual ~UsbHidOutput() = default;
  virtual bool send_report(const std::uint8_t* bytes, std::uint16_t size) = 0;
};

class UsbConfigurationTransport {
 public:
  virtual ~UsbConfigurationTransport() = default;
  virtual void poll() = 0;
};

class KeymapStorage {
 public:
  virtual ~KeymapStorage() = default;
  virtual bool read(std::uint8_t* destination, std::uint16_t capacity,
                    std::uint16_t& bytes_read) = 0;
  virtual bool write(const std::uint8_t* source, std::uint16_t size) = 0;
};

}  // namespace kohlrausch::ports
