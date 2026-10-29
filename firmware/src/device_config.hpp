#pragma once

#include <array>
#include <cstddef>
#include <cstdint>

namespace kohlrausch {

inline constexpr std::uint32_t kConfigMagic = 0x3146434B;  // "KCF1"
inline constexpr std::uint16_t kConfigSchemaVersion = 1;
inline constexpr std::uint8_t kConfigLayerCount = 4;
inline constexpr std::uint8_t kConfigKeyCount = 9;
inline constexpr std::uint8_t kConfigMaxMacros = 8;
inline constexpr std::uint8_t kConfigMaxMacroSteps = 64;
inline constexpr std::uint16_t kConfigMaxBytes = 8192;
inline constexpr std::size_t kConfigHeaderSize = 12;
inline constexpr std::size_t kConfigLayerSize =
    kConfigKeyCount * 8;  // nine eight-byte bindings
inline constexpr std::size_t kConfigMacroHeaderSize = 12;
inline constexpr std::size_t kConfigMacroStepSize = 12;

enum class BindingKind : std::uint8_t {
  None = 0,
  Chord = 1,
  Mouse = 2,
  Macro = 3,
};

enum class MouseAction : std::uint8_t {
  None = 0,
  LeftClick = 1,
  RightClick = 2,
  DoubleClick = 3,
  ScrollUp = 4,
  ScrollDown = 5,
};

enum class MacroLoopMode : std::uint8_t {
  Once = 0,
  UntilStop = 1,
  UntilAnyClick = 2,
  Count = 3,
};

enum class MacroTiming : std::uint8_t {
  Fixed = 0,
  Recorded = 1,
};

struct KeyBinding {
  std::uint8_t kind = 0;
  std::uint8_t modifiers = 0;
  std::uint8_t keycode = 0;
  std::uint8_t macro_index = 0;
  std::uint8_t mouse_action = 0;
  std::array<std::uint8_t, 3> reserved{};
};

struct Layer {
  std::array<KeyBinding, kConfigKeyCount> keys{};
};

struct MacroStep {
  std::uint32_t delay_ms = 0;
  std::uint8_t kind = 0;
  std::uint8_t modifiers = 0;
  std::uint8_t keycode = 0;
  std::uint8_t mouse_action = 0;
  std::array<std::uint8_t, 4> reserved{};
};

struct Macro {
  std::uint8_t step_count = 0;
  std::uint8_t loop_mode = 0;
  std::uint16_t loop_count = 0;
  std::uint32_t fixed_delay_ms = 0;
  std::uint8_t timing = 0;
  std::array<std::uint8_t, 3> reserved{};
  std::array<MacroStep, kConfigMaxMacroSteps> steps{};
};

struct DeviceConfig {
  std::uint8_t schema_version = kConfigSchemaVersion;
  std::uint8_t layer_count = kConfigLayerCount;
  std::uint8_t macro_count = 0;
  std::array<Layer, kConfigLayerCount> layers{};
  std::array<Macro, kConfigMaxMacros> macros{};
};

DeviceConfig default_config() noexcept;

std::size_t serialized_config_size(const DeviceConfig& config) noexcept;
std::size_t serialize_config(const DeviceConfig& config, std::uint8_t* destination,
                             std::size_t capacity) noexcept;
bool parse_config(const std::uint8_t* data, std::size_t size, DeviceConfig& output,
                  char* error, std::size_t error_size) noexcept;

std::uint32_t crc32(const std::uint8_t* data, std::size_t size) noexcept;

}  // namespace kohlrausch
