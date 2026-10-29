#include "../src/layer_logic.hpp"
#include "../src/runtime_logic.hpp"
#include "../include/hardware_ports.hpp"

#include <cassert>
#include <cstdint>
#include <limits>

using kohlrausch::DebouncedPress;
using kohlrausch::FirmwareRuntime;
using kohlrausch::LayerManager;
using kohlrausch::OledCycle;
using kohlrausch::OledScreen;

int main() {
  static_assert(static_cast<std::uint8_t>(kohlrausch::ports::KeyId::Key3_1) == 0);
  static_assert(static_cast<std::uint8_t>(kohlrausch::ports::KeyId::Key1_3) == 8);

  // Four-layer sequence and safe handling of invalid persisted layer values.
  assert(kohlrausch::next_layer(1) == 2);
  assert(kohlrausch::next_layer(2) == 3);
  assert(kohlrausch::next_layer(3) == 4);
  assert(kohlrausch::next_layer(4) == 1);
  assert(kohlrausch::next_layer(0) == 1);

  LayerManager layers;
  assert(layers.active_layer() == 1);
  assert(layers.cycle() == 2);
  assert(layers.select(4));
  assert(layers.active_layer() == 4);
  assert(!layers.select(0));
  assert(!layers.select(5));
  assert(layers.active_layer() == 4);

  // Bounce is ignored, one event is emitted at the stable press edge, and a
  // stable release is required before another press can be emitted.
  DebouncedPress button(20);
  assert(!button.update(false, 0));
  assert(!button.update(true, 5));
  assert(!button.update(false, 12));
  assert(!button.update(true, 20));
  assert(!button.update(true, 39));
  assert(button.update(true, 40));
  assert(!button.update(true, 80));
  assert(!button.update(false, 90));
  assert(!button.update(false, 109));
  assert(!button.update(false, 110));
  assert(!button.update(true, 120));
  assert(button.update(true, 140));

  // A held switch during boot seeds the debouncer without advancing a layer.
  DebouncedPress held_at_boot(20);
  assert(!held_at_boot.update(true, 0));
  assert(!held_at_boot.update(true, 100));

  // Unsigned elapsed-time arithmetic remains correct across timer rollover.
  DebouncedPress rollover_button(20);
  const auto near_wrap = std::numeric_limits<std::uint32_t>::max() - 5;
  assert(!rollover_button.update(false, near_wrap - 1));
  assert(!rollover_button.update(true, near_wrap));
  assert(rollover_button.update(true, 14));

  // The OLED presentation alternates non-blockingly every five seconds.
  OledCycle display;
  display.begin(3, 0);
  assert(display.frame().screen == OledScreen::LayerSelector);
  assert(display.frame().active_layer == 3);
  assert(!display.update(4999));
  assert(display.update(5000));
  assert(display.frame().screen == OledScreen::Branding);
  assert(display.frame().active_layer == 3);
  assert(!display.update(9999));
  assert(display.update(10000));
  assert(display.frame().screen == OledScreen::LayerSelector);
  assert(display.update(25000));
  assert(display.frame().screen == OledScreen::Branding);

  // A layer change immediately selects its screen and restarts the full cycle.
  assert(display.layer_changed(2, 26000));
  assert(display.frame().screen == OledScreen::LayerSelector);
  assert(display.frame().active_layer == 2);
  assert(!display.update(30999));
  assert(display.update(31000));
  assert(display.frame().screen == OledScreen::Branding);
  assert(!display.layer_changed(0, 32000));
  assert(display.frame().active_layer == 2);

  OledCycle rollover_display;
  rollover_display.begin(1, near_wrap);
  assert(!rollover_display.update(near_wrap + 4999));
  assert(rollover_display.frame().screen == OledScreen::LayerSelector);
  assert(rollover_display.update(near_wrap + 5000));
  assert(rollover_display.frame().screen == OledScreen::Branding);

  // Integrated runtime starts on layer 1, debounces LYR, and restarts OLED.
  FirmwareRuntime runtime;
  runtime.begin(false, 0);
  auto result = runtime.update(false, 0);
  assert(result.active_layer == 1);
  assert(result.oled_frame_changed);
  assert(result.oled_frame.screen == OledScreen::LayerSelector);

  result = runtime.update(true, 10);
  assert(!result.layer_changed);
  result = runtime.update(true, 309);
  assert(!result.layer_changed);
  result = runtime.update(true, 310);
  assert(result.layer_changed);
  assert(result.active_layer == 2);
  assert(result.oled_frame_changed);
  assert(result.oled_frame.screen == OledScreen::LayerSelector);
  result = runtime.update(true, 400);
  assert(!result.layer_changed);
  assert(result.active_layer == 2);

  result = runtime.update(false, 410);
  assert(!result.layer_changed);
  result = runtime.update(false, 730);
  assert(!result.layer_changed);
  runtime.update(true, 740);
  result = runtime.update(true, 1040);
  assert(result.layer_changed);
  assert(result.active_layer == 3);
}
