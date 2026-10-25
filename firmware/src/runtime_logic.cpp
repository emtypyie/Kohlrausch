#include "runtime_logic.hpp"

#include "layer_logic.hpp"

namespace kohlrausch {

std::uint8_t LayerManager::active_layer() const noexcept { return active_layer_; }

std::uint8_t LayerManager::cycle() noexcept {
  active_layer_ = next_layer(active_layer_);
  return active_layer_;
}

bool LayerManager::select(std::uint8_t layer) noexcept {
  if (layer < 1 || layer > kLayerCount) return false;
  active_layer_ = layer;
  return true;
}

DebouncedPress::DebouncedPress(std::uint32_t debounce_ms) noexcept
    : debounce_ms_(debounce_ms) {}

bool DebouncedPress::update(bool raw_pressed, std::uint32_t now_ms) noexcept {
  if (!initialized_) {
    initialized_ = true;
    candidate_pressed_ = raw_pressed;
    stable_pressed_ = raw_pressed;
    candidate_since_ms_ = now_ms;
    return false;
  }

  if (raw_pressed != candidate_pressed_) {
    candidate_pressed_ = raw_pressed;
    candidate_since_ms_ = now_ms;
    return false;
  }

  if (candidate_pressed_ == stable_pressed_ ||
      static_cast<std::uint32_t>(now_ms - candidate_since_ms_) < debounce_ms_) {
    return false;
  }

  stable_pressed_ = candidate_pressed_;
  return stable_pressed_;
}

void OledCycle::begin(std::uint8_t active_layer, std::uint32_t now_ms) noexcept {
  active_layer_ = active_layer >= 1 && active_layer <= kLayerCount
                      ? active_layer
                      : kStartupLayer;
  screen_ = OledScreen::LayerSelector;
  screen_started_ms_ = now_ms;
  initialized_ = true;
}

bool OledCycle::layer_changed(std::uint8_t active_layer,
                              std::uint32_t now_ms) noexcept {
  if (active_layer < 1 || active_layer > kLayerCount) return false;
  active_layer_ = active_layer;
  screen_ = OledScreen::LayerSelector;
  screen_started_ms_ = now_ms;
  initialized_ = true;
  return true;
}

bool OledCycle::update(std::uint32_t now_ms) noexcept {
  if (!initialized_) {
    begin(kStartupLayer, now_ms);
    return true;
  }

  const auto elapsed = static_cast<std::uint32_t>(now_ms - screen_started_ms_);
  const auto transitions = elapsed / kOledScreenDurationMs;
  if (transitions == 0) return false;

  if ((transitions % 2) != 0) {
    screen_ = screen_ == OledScreen::LayerSelector ? OledScreen::Branding
                                                    : OledScreen::LayerSelector;
  }
  screen_started_ms_ += transitions * kOledScreenDurationMs;
  return true;
}

OledFrame OledCycle::frame() const noexcept { return {screen_, active_layer_}; }

void FirmwareRuntime::begin(bool lyr_pressed, std::uint32_t now_ms) noexcept {
  lyr_button_.update(lyr_pressed, now_ms);  // Seed input state; never advance at boot.
  oled_cycle_.begin(layers_.active_layer(), now_ms);
  initialized_ = true;
  frame_dirty_ = true;
}

RuntimeUpdate FirmwareRuntime::update(bool lyr_pressed,
                                      std::uint32_t now_ms) noexcept {
  if (!initialized_) begin(lyr_pressed, now_ms);

  bool layer_changed = false;
  bool frame_changed = frame_dirty_;
  frame_dirty_ = false;
  if (lyr_button_.update(lyr_pressed, now_ms)) {
    layers_.cycle();
    oled_cycle_.layer_changed(layers_.active_layer(), now_ms);
    layer_changed = true;
    frame_dirty_ = true;
  } else {
    frame_dirty_ = oled_cycle_.update(now_ms) || frame_dirty_;
  }

  frame_changed = frame_changed || frame_dirty_;
  frame_dirty_ = false;
  return {layers_.active_layer(), oled_cycle_.frame(), layer_changed,
          frame_changed};
}

}  // namespace kohlrausch
