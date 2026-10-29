#include "../src/device_config.hpp"

#include <array>
#include <cassert>
#include <cstddef>
#include <cstdint>
#include <cstring>

using namespace kohlrausch;

namespace {

KeyBinding chord(std::uint8_t modifiers, std::uint8_t keycode) {
  KeyBinding binding{};
  binding.kind = static_cast<std::uint8_t>(BindingKind::Chord);
  binding.modifiers = modifiers;
  binding.keycode = keycode;
  return binding;
}

KeyBinding mouse(MouseAction action) {
  KeyBinding binding{};
  binding.kind = static_cast<std::uint8_t>(BindingKind::Mouse);
  binding.mouse_action = static_cast<std::uint8_t>(action);
  return binding;
}

MacroStep chord_step(std::uint32_t delay_ms, std::uint8_t keycode) {
  MacroStep step{};
  step.delay_ms = delay_ms;
  step.kind = static_cast<std::uint8_t>(BindingKind::Chord);
  step.keycode = keycode;
  return step;
}

MacroStep mouse_step(std::uint32_t delay_ms, MouseAction action) {
  MacroStep step{};
  step.delay_ms = delay_ms;
  step.kind = static_cast<std::uint8_t>(BindingKind::Mouse);
  step.mouse_action = static_cast<std::uint8_t>(action);
  return step;
}

void test_default_round_trip() {
  const auto config = default_config();
  std::array<std::uint8_t, kConfigMaxBytes> bytes{};
  const auto size = serialize_config(config, bytes.data(), bytes.size());
  assert(size == kConfigHeaderSize + kConfigLayerCount * kConfigLayerSize);

  DeviceConfig parsed{};
  char error[32]{};
  assert(parse_config(bytes.data(), size, parsed, error, sizeof(error)));
  assert(parsed.schema_version == kConfigSchemaVersion);
  assert(parsed.layer_count == kConfigLayerCount);
  assert(parsed.macro_count == 0);
  for (const auto& layer : parsed.layers) {
    for (const auto& binding : layer.keys) {
      assert(binding.kind == static_cast<std::uint8_t>(BindingKind::None));
    }
  }
}

void test_populated_round_trip() {
  auto config = default_config();
  config.layers[0].keys[0] = chord(0x02, 0x04);
  config.layers[0].keys[1] = mouse(MouseAction::LeftClick);
  config.layers[0].keys[2].kind =
      static_cast<std::uint8_t>(BindingKind::Macro);
  config.layers[0].keys[2].macro_index = 0;
  config.layers[3].keys[8] = chord(0x01, 0x2C);

  config.macro_count = 1;
  auto& macro = config.macros[0];
  macro.step_count = 2;
  macro.loop_mode = static_cast<std::uint8_t>(MacroLoopMode::Count);
  macro.loop_count = 2;
  macro.fixed_delay_ms = 50;
  macro.timing = static_cast<std::uint8_t>(MacroTiming::Fixed);
  macro.steps[0] = chord_step(10, 0x05);
  macro.steps[1] = mouse_step(20, MouseAction::ScrollUp);

  std::array<std::uint8_t, kConfigMaxBytes> bytes{};
  const auto size = serialize_config(config, bytes.data(), bytes.size());
  assert(size > kConfigHeaderSize + kConfigLayerCount * kConfigLayerSize);
  assert(size == serialized_config_size(config));

  DeviceConfig parsed{};
  char error[32]{};
  assert(parse_config(bytes.data(), size, parsed, error, sizeof(error)));
  assert(parsed.layers[0].keys[0].kind ==
         static_cast<std::uint8_t>(BindingKind::Chord));
  assert(parsed.layers[0].keys[0].modifiers == 0x02);
  assert(parsed.layers[0].keys[0].keycode == 0x04);
  assert(parsed.layers[0].keys[1].mouse_action ==
         static_cast<std::uint8_t>(MouseAction::LeftClick));
  assert(parsed.layers[0].keys[2].macro_index == 0);
  assert(parsed.layers[3].keys[8].keycode == 0x2C);
  assert(parsed.macro_count == 1);
  assert(parsed.macros[0].step_count == 2);
  assert(parsed.macros[0].loop_mode ==
         static_cast<std::uint8_t>(MacroLoopMode::Count));
  assert(parsed.macros[0].loop_count == 2);
  assert(parsed.macros[0].fixed_delay_ms == 50);
  assert(parsed.macros[0].steps[0].keycode == 0x05);
  assert(parsed.macros[0].steps[1].mouse_action ==
         static_cast<std::uint8_t>(MouseAction::ScrollUp));
}

void test_crc() {
  const char* text = "123456789";
  assert(crc32(reinterpret_cast<const std::uint8_t*>(text), 9) == 0xCBF43926U);
}

void test_rejects_malformed() {
  auto config = default_config();
  std::array<std::uint8_t, kConfigMaxBytes> bytes{};
  const auto size = serialize_config(config, bytes.data(), bytes.size());
  char error[32]{};

  auto copy = bytes;
  copy[0] ^= 0xFF;
  assert(!parse_config(copy.data(), size, config, error, sizeof(error)));
  assert(std::strcmp(error, "config-magic") == 0);

  copy = bytes;
  copy[6] = 3;
  assert(!parse_config(copy.data(), size, config, error, sizeof(error)));
  assert(std::strcmp(error, "config-layer-count") == 0);

  copy = bytes;
  copy[size] = 1;
  copy[8] = static_cast<std::uint8_t>((size + 1) & 0xFF);
  copy[9] = static_cast<std::uint8_t>(((size + 1) >> 8) & 0xFF);
  assert(!parse_config(copy.data(), size + 1, config, error, sizeof(error)));
  assert(std::strcmp(error, "config-trailing-data") == 0);

  copy = bytes;
  copy[kConfigHeaderSize + 1] = 1;
  copy[kConfigHeaderSize + 2] = 1;
  assert(!parse_config(copy.data(), size, config, error, sizeof(error)));
  assert(std::strcmp(error, "config-binding") == 0);

  auto macro_config = default_config();
  macro_config.macro_count = 1;
  macro_config.macros[0].step_count = 1;
  macro_config.macros[0].steps[0] = chord_step(10, 0x05);
  const auto macro_size =
      serialize_config(macro_config, bytes.data(), bytes.size());
  copy = bytes;
  copy[7] = 0;
  assert(!parse_config(copy.data(), macro_size, macro_config, error,
                       sizeof(error)));
  assert(std::strcmp(error, "config-trailing-data") == 0);
}

void test_macro_timing_modes() {
  const auto macro_start = kConfigHeaderSize + kConfigLayerCount * kConfigLayerSize;

  // 1. Verify MacroTiming::Fixed wire value is 0
  {
    auto config = default_config();
    config.macro_count = 1;
    auto& macro = config.macros[0];
    macro.step_count = 1;
    macro.loop_mode = static_cast<std::uint8_t>(MacroLoopMode::Once);
    macro.timing = static_cast<std::uint8_t>(MacroTiming::Fixed);
    macro.fixed_delay_ms = 50;
    macro.steps[0] = chord_step(10, 0x05);

    std::array<std::uint8_t, kConfigMaxBytes> bytes{};
    const auto size = serialize_config(config, bytes.data(), bytes.size());
    assert(bytes[macro_start + 8] == 0);  // Fixed timing wire value MUST be 0

    DeviceConfig parsed{};
    char error[32]{};
    assert(parse_config(bytes.data(), size, parsed, error, sizeof(error)));
    assert(parsed.macros[0].timing ==
           static_cast<std::uint8_t>(MacroTiming::Fixed));
    assert(parsed.macros[0].fixed_delay_ms == 50);
  }

  // 2. Verify MacroTiming::Recorded wire value is 1
  {
    auto config = default_config();
    config.macro_count = 1;
    auto& macro = config.macros[0];
    macro.step_count = 1;
    macro.loop_mode = static_cast<std::uint8_t>(MacroLoopMode::Once);
    macro.timing = static_cast<std::uint8_t>(MacroTiming::Recorded);
    macro.fixed_delay_ms = 0;
    macro.steps[0] = chord_step(42, 0x04);

    std::array<std::uint8_t, kConfigMaxBytes> bytes{};
    const auto size = serialize_config(config, bytes.data(), bytes.size());
    assert(bytes[macro_start + 8] == 1);  // Recorded timing wire value MUST be 1

    DeviceConfig parsed{};
    char error[32]{};
    assert(parse_config(bytes.data(), size, parsed, error, sizeof(error)));
    assert(parsed.macros[0].timing ==
           static_cast<std::uint8_t>(MacroTiming::Recorded));
    assert(parsed.macros[0].steps[0].delay_ms == 42);
  }
}

}  // namespace

int main() {
  static_assert(sizeof(KeyBinding) == 8);
  static_assert(sizeof(MacroStep) == 12);
  static_assert(sizeof(Macro) == 780);
  static_assert(sizeof(DeviceConfig) == 6532);

  test_default_round_trip();
  test_populated_round_trip();
  test_macro_timing_modes();
  test_crc();
  test_rejects_malformed();
}
