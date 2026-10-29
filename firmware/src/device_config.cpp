#include "device_config.hpp"

#include <algorithm>
#include <cstring>

namespace kohlrausch {
namespace {

void write_u16(std::uint8_t* destination, std::uint16_t value) {
  destination[0] = static_cast<std::uint8_t>(value & 0xFF);
  destination[1] = static_cast<std::uint8_t>((value >> 8) & 0xFF);
}

void write_u32(std::uint8_t* destination, std::uint32_t value) {
  destination[0] = static_cast<std::uint8_t>(value & 0xFF);
  destination[1] = static_cast<std::uint8_t>((value >> 8) & 0xFF);
  destination[2] = static_cast<std::uint8_t>((value >> 16) & 0xFF);
  destination[3] = static_cast<std::uint8_t>((value >> 24) & 0xFF);
}

std::uint16_t read_u16(const std::uint8_t* source) {
  return static_cast<std::uint16_t>(source[0]) |
         static_cast<std::uint16_t>(static_cast<std::uint16_t>(source[1]) << 8);
}

std::uint32_t read_u32(const std::uint8_t* source) {
  return static_cast<std::uint32_t>(source[0]) |
         (static_cast<std::uint32_t>(source[1]) << 8) |
         (static_cast<std::uint32_t>(source[2]) << 16) |
         (static_cast<std::uint32_t>(source[3]) << 24);
}

bool zero(const std::uint8_t* data, std::size_t size) {
  for (std::size_t index = 0; index < size; ++index) {
    if (data[index] != 0) return false;
  }
  return true;
}

bool set_error(char* error, std::size_t error_size, const char* message) {
  if (error != nullptr && error_size != 0) {
    std::strncpy(error, message, error_size - 1);
    error[error_size - 1] = '\0';
  }
  return false;
}

bool valid_binding(const KeyBinding& binding, std::uint8_t macro_count) {
  if (!zero(binding.reserved.data(), binding.reserved.size())) return false;
  switch (static_cast<BindingKind>(binding.kind)) {
    case BindingKind::None:
      return binding.modifiers == 0 && binding.keycode == 0 &&
             binding.macro_index == 0 && binding.mouse_action == 0;
    case BindingKind::Chord:
      return (binding.modifiers & 0xF0U) == 0 && binding.keycode != 0 &&
             binding.macro_index == 0 && binding.mouse_action == 0;
    case BindingKind::Mouse:
      return binding.modifiers == 0 && binding.keycode == 0 &&
             binding.macro_index == 0 && binding.mouse_action >= 1 &&
             binding.mouse_action <= 5;
    case BindingKind::Macro:
      return binding.modifiers == 0 && binding.keycode == 0 &&
             binding.mouse_action == 0 && binding.macro_index < macro_count;
    default:
      return false;
  }
}

bool valid_step(const MacroStep& step) {
  if (step.delay_ms > 600000) return false;
  if (!zero(step.reserved.data(), step.reserved.size())) return false;
  switch (static_cast<BindingKind>(step.kind)) {
    case BindingKind::Chord:
      return (step.modifiers & 0xF0U) == 0 && step.keycode != 0 &&
             step.mouse_action == 0;
    case BindingKind::Mouse:
      return step.modifiers == 0 && step.keycode == 0 &&
             step.mouse_action >= 1 && step.mouse_action <= 5;
    default:
      return false;
  }
}

}  // namespace

DeviceConfig default_config() noexcept { return DeviceConfig{}; }

std::size_t serialized_config_size(const DeviceConfig& config) noexcept {
  std::size_t size = kConfigHeaderSize + kConfigLayerCount * kConfigLayerSize;
  const auto macro_count = std::min<std::uint8_t>(config.macro_count, kConfigMaxMacros);
  for (std::uint8_t index = 0; index < macro_count; ++index) {
    const auto step_count =
        std::min<std::uint8_t>(config.macros[index].step_count,
                               kConfigMaxMacroSteps);
    size += kConfigMacroHeaderSize +
            static_cast<std::size_t>(step_count) * kConfigMacroStepSize;
  }
  return size;
}

std::size_t serialize_config(const DeviceConfig& config, std::uint8_t* destination,
                             std::size_t capacity) noexcept {
  const std::size_t size = serialized_config_size(config);
  if (destination == nullptr || size > capacity) return 0;

  std::memset(destination, 0, size);
  write_u32(destination, kConfigMagic);
  write_u16(destination + 4, kConfigSchemaVersion);
  destination[6] = kConfigLayerCount;
  destination[7] = std::min<std::uint8_t>(config.macro_count, kConfigMaxMacros);
  write_u16(destination + 8, static_cast<std::uint16_t>(size));
  write_u16(destination + 10, 0);

  std::size_t cursor = kConfigHeaderSize;
  for (std::size_t layer = 0; layer < kConfigLayerCount; ++layer) {
    for (std::size_t key = 0; key < kConfigKeyCount; ++key) {
      const auto& binding = config.layers[layer].keys[key];
      destination[cursor++] = binding.kind;
      destination[cursor++] = binding.modifiers;
      destination[cursor++] = binding.keycode;
      destination[cursor++] = binding.macro_index;
      destination[cursor++] = binding.mouse_action;
      cursor += 3;
    }
  }

  const auto macro_count = std::min<std::uint8_t>(config.macro_count, kConfigMaxMacros);
  for (std::uint8_t index = 0; index < macro_count; ++index) {
    const auto& macro = config.macros[index];
    destination[cursor++] =
        std::min<std::uint8_t>(macro.step_count, kConfigMaxMacroSteps);
    destination[cursor++] = macro.loop_mode;
    write_u16(destination + cursor, macro.loop_count);
    cursor += 2;
    write_u32(destination + cursor, macro.fixed_delay_ms);
    cursor += 4;
    destination[cursor++] = macro.timing;
    cursor += 3;
    const auto step_count =
        std::min<std::uint8_t>(macro.step_count, kConfigMaxMacroSteps);
    for (std::uint8_t step_index = 0; step_index < step_count; ++step_index) {
      const auto& step = macro.steps[step_index];
      write_u32(destination + cursor, step.delay_ms);
      cursor += 4;
      destination[cursor++] = step.kind;
      destination[cursor++] = step.modifiers;
      destination[cursor++] = step.keycode;
      destination[cursor++] = step.mouse_action;
      cursor += 4;
    }
  }
  return cursor;
}

bool parse_config(const std::uint8_t* data, std::size_t size, DeviceConfig& output,
                  char* error, std::size_t error_size) noexcept {
  if (data == nullptr || size < kConfigHeaderSize) {
    return set_error(error, error_size, "config-too-short");
  }
  if (read_u32(data) != kConfigMagic) {
    return set_error(error, error_size, "config-magic");
  }
  if (read_u16(data + 4) != kConfigSchemaVersion) {
    return set_error(error, error_size, "config-schema");
  }
  if (data[6] != kConfigLayerCount) {
    return set_error(error, error_size, "config-layer-count");
  }
  const std::uint8_t macro_count = data[7];
  if (macro_count > kConfigMaxMacros) {
    return set_error(error, error_size, "config-macro-count");
  }
  if (read_u16(data + 8) != size) {
    return set_error(error, error_size, "config-size");
  }
  if (read_u16(data + 10) != 0) {
    return set_error(error, error_size, "config-reserved");
  }

  std::size_t cursor = kConfigHeaderSize;
  if (size < cursor + kConfigLayerCount * kConfigLayerSize) {
    return set_error(error, error_size, "config-layers-truncated");
  }

  DeviceConfig candidate{};
  candidate.schema_version = kConfigSchemaVersion;
  candidate.layer_count = kConfigLayerCount;
  candidate.macro_count = macro_count;

  for (std::size_t layer = 0; layer < kConfigLayerCount; ++layer) {
    for (std::size_t key = 0; key < kConfigKeyCount; ++key) {
      auto& binding = candidate.layers[layer].keys[key];
      binding.kind = data[cursor++];
      binding.modifiers = data[cursor++];
      binding.keycode = data[cursor++];
      binding.macro_index = data[cursor++];
      binding.mouse_action = data[cursor++];
      std::memcpy(binding.reserved.data(), data + cursor, 3);
      cursor += 3;
      if (!valid_binding(binding, macro_count)) {
        return set_error(error, error_size, "config-binding");
      }
    }
  }

  for (std::uint8_t index = 0; index < macro_count; ++index) {
    if (cursor + kConfigMacroHeaderSize > size) {
      return set_error(error, error_size, "config-macro-truncated");
    }
    auto& macro = candidate.macros[index];
    macro.step_count = data[cursor++];
    macro.loop_mode = data[cursor++];
    macro.loop_count = read_u16(data + cursor);
    cursor += 2;
    macro.fixed_delay_ms = read_u32(data + cursor);
    cursor += 4;
    macro.timing = data[cursor++];
    std::memcpy(macro.reserved.data(), data + cursor, 3);
    cursor += 3;

    if (macro.step_count == 0 || macro.step_count > kConfigMaxMacroSteps) {
      return set_error(error, error_size, "config-step-count");
    }
    if (macro.loop_mode > static_cast<std::uint8_t>(MacroLoopMode::Count)) {
      return set_error(error, error_size, "config-loop-mode");
    }
    if (macro.timing > static_cast<std::uint8_t>(MacroTiming::Recorded)) {
      return set_error(error, error_size, "config-timing");
    }
    if (macro.fixed_delay_ms > 600000) {
      return set_error(error, error_size, "config-fixed-delay");
    }
    if (!zero(macro.reserved.data(), macro.reserved.size())) {
      return set_error(error, error_size, "config-macro-reserved");
    }
    const auto loop_mode = static_cast<MacroLoopMode>(macro.loop_mode);
    if (loop_mode == MacroLoopMode::Count) {
      if (macro.loop_count == 0 || macro.loop_count > 10000) {
        return set_error(error, error_size, "config-loop-count");
      }
    } else if (macro.loop_count != 0) {
      return set_error(error, error_size, "config-loop-count");
    }

    if (cursor + static_cast<std::size_t>(macro.step_count) *
                     kConfigMacroStepSize >
        size) {
      return set_error(error, error_size, "config-steps-truncated");
    }
    for (std::uint8_t step_index = 0; step_index < macro.step_count; ++step_index) {
      auto& step = macro.steps[step_index];
      step.delay_ms = read_u32(data + cursor);
      cursor += 4;
      step.kind = data[cursor++];
      step.modifiers = data[cursor++];
      step.keycode = data[cursor++];
      step.mouse_action = data[cursor++];
      std::memcpy(step.reserved.data(), data + cursor, 4);
      cursor += 4;
      if (!valid_step(step)) {
        return set_error(error, error_size, "config-step");
      }
    }
  }

  if (cursor != size) {
    return set_error(error, error_size, "config-trailing-data");
  }

  output = candidate;
  if (error != nullptr && error_size != 0) error[0] = '\0';
  return true;
}

std::uint32_t crc32(const std::uint8_t* data, std::size_t size) noexcept {
  std::uint32_t crc = 0xFFFFFFFFU;
  for (std::size_t index = 0; index < size; ++index) {
    crc ^= data[index];
    for (std::uint8_t bit = 0; bit < 8; ++bit) {
      crc = (crc >> 1) ^ (0xEDB88320U & (0U - (crc & 1U)));
    }
  }
  return ~crc;
}

}  // namespace kohlrausch
