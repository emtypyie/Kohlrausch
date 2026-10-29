#pragma once

#include <cstdint>

namespace kohlrausch {

inline constexpr std::uint8_t kStartupLayer = 1;
inline constexpr std::uint8_t kLayerCount = 4;
inline constexpr std::uint32_t kLyrDebounceMs = 300;
inline constexpr std::uint32_t kOledScreenDurationMs = 5000;

enum class OledScreen : std::uint8_t {
  LayerSelector,
  Branding,
};

struct OledFrame {
  OledScreen screen;
  std::uint8_t active_layer;
};

class LayerManager {
 public:
  std::uint8_t active_layer() const noexcept;
  std::uint8_t cycle() noexcept;
  bool select(std::uint8_t layer) noexcept;

 private:
  std::uint8_t active_layer_ = kStartupLayer;
};

// Accepts a normalized semantic pressed state; GPIO polarity is handled by the
// board-specific input driver.
class DebouncedPress {
 public:
  explicit DebouncedPress(std::uint32_t debounce_ms = kLyrDebounceMs) noexcept;

  // Returns true once for each stable transition to pressed.
  bool update(bool raw_pressed, std::uint32_t now_ms) noexcept;

 private:
  std::uint32_t debounce_ms_;
  std::uint32_t candidate_since_ms_ = 0;
  bool initialized_ = false;
  bool candidate_pressed_ = false;
  bool stable_pressed_ = false;
};

class OledCycle {
 public:
  void begin(std::uint8_t active_layer, std::uint32_t now_ms) noexcept;

  // Immediately selects the layer screen and starts a fresh five-second cycle.
  bool layer_changed(std::uint8_t active_layer, std::uint32_t now_ms) noexcept;

  // Advances by elapsed time without sleeping or doing display I/O.
  bool update(std::uint32_t now_ms) noexcept;
  OledFrame frame() const noexcept;

 private:
  std::uint8_t active_layer_ = kStartupLayer;
  OledScreen screen_ = OledScreen::LayerSelector;
  std::uint32_t screen_started_ms_ = 0;
  bool initialized_ = false;
};

struct RuntimeUpdate {
  std::uint8_t active_layer;
  OledFrame oled_frame;
  bool layer_changed;
  bool oled_frame_changed;
};

// Hardware-independent runtime state. A board loop supplies normalized LYR
// input and renders only when oled_frame_changed is true.
class FirmwareRuntime {
 public:
  void begin(bool lyr_pressed, std::uint32_t now_ms) noexcept;
  RuntimeUpdate update(bool lyr_pressed, std::uint32_t now_ms) noexcept;

 private:
  LayerManager layers_;
  DebouncedPress lyr_button_;
  OledCycle oled_cycle_;
  bool initialized_ = false;
  bool frame_dirty_ = false;
};

}  // namespace kohlrausch
