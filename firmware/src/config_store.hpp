#pragma once

#include "device_config.hpp"

#include <array>
#include <cstddef>
#include <cstdint>

namespace kohlrausch {

struct StoredConfigInfo {
  bool from_flash = false;
  std::uint32_t sequence = 0;
  std::uint32_t crc = 0;
  std::uint16_t size = 0;
  std::uint8_t slot = 0;
};

class ConfigStore {
 public:
  bool load(DeviceConfig& output, StoredConfigInfo& info, char* error,
            std::size_t error_size);
  bool save(const DeviceConfig& config, StoredConfigInfo& info, char* error,
            std::size_t error_size);
  bool verify_last_write(char* error, std::size_t error_size) const;

 private:
  static constexpr std::uint32_t kSlotSize = 8192;
  static constexpr std::uint32_t kSlotHeaderSize = 20;
  static constexpr std::uint32_t kSlotMagic = 0x314C534B;  // "KSL1"
  static constexpr std::uint16_t kSlotSchema = 1;

  std::array<std::uint8_t, kSlotSize> slot_buffer_{};
  std::uint32_t last_sequence_ = 0;
  std::uint8_t last_slot_ = 0;
  bool have_stored_ = false;
};

}  // namespace kohlrausch
