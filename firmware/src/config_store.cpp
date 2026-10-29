#include "config_store.hpp"

#include "hardware/flash.h"
#include "hardware/regs/addressmap.h"
#include "pico/flash.h"
#include "pico/platform.h"

#include <algorithm>
#include <cstring>

namespace kohlrausch {
namespace {

constexpr std::uint32_t kSlotAOffset =
    static_cast<std::uint32_t>(PICO_FLASH_SIZE_BYTES) - 2 * 8192;
constexpr std::uint32_t kSlotBOffset =
    static_cast<std::uint32_t>(PICO_FLASH_SIZE_BYTES) - 8192;

static_assert(PICO_FLASH_SIZE_BYTES >= 4 * 8192,
              "RP2040 flash must leave room for two configuration slots");

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

bool set_error(char* error, std::size_t error_size, const char* message) {
  if (error != nullptr && error_size != 0) {
    std::strncpy(error, message, error_size - 1);
    error[error_size - 1] = '\0';
  }
  return false;
}

struct FlashWriteJob {
  std::uint32_t offset;
  const std::uint8_t* data;
  std::size_t size;
};

void __no_inline_not_in_flash_func(perform_flash_write)(void* param) {
  auto* job = static_cast<FlashWriteJob*>(param);
  flash_range_erase(job->offset, job->size);
  flash_range_program(job->offset, job->data, job->size);
}

bool read_slot(std::uint32_t offset, DeviceConfig& output, StoredConfigInfo& info,
               char* error, std::size_t error_size) {
  const auto* raw = reinterpret_cast<const std::uint8_t*>(XIP_BASE + offset);
  if (read_u32(raw) != 0x314C534B) return false;
  if (read_u16(raw + 4) != 1) return false;
  if (read_u16(raw + 6) != 0) return false;
  const auto payload_size = read_u16(raw + 12);
  if (read_u16(raw + 14) != 0) return false;
  if (payload_size == 0 || payload_size > kConfigMaxBytes ||
      payload_size + 20 > 8192) {
    return false;
  }
  const auto expected_crc = read_u32(raw + 16);
  if (crc32(raw + 20, payload_size) != expected_crc) return false;
  if (!parse_config(raw + 20, payload_size, output, error, error_size)) {
    return false;
  }
  info.from_flash = true;
  info.sequence = read_u32(raw + 8);
  info.crc = expected_crc;
  info.size = payload_size;
  info.slot = offset == kSlotAOffset ? 0 : 1;
  return true;
}

}  // namespace

bool ConfigStore::load(DeviceConfig& output, StoredConfigInfo& info, char* error,
                       std::size_t error_size) {
  DeviceConfig first{};
  DeviceConfig second{};
  StoredConfigInfo first_info{};
  StoredConfigInfo second_info{};
  const bool first_valid =
      read_slot(kSlotAOffset, first, first_info, error, error_size);
  const bool second_valid =
      read_slot(kSlotBOffset, second, second_info, error, error_size);

  if (first_valid && second_valid) {
    if (static_cast<std::int32_t>(second_info.sequence - first_info.sequence) > 0) {
      output = second;
      info = second_info;
    } else {
      output = first;
      info = first_info;
    }
  } else if (first_valid) {
    output = first;
    info = first_info;
  } else if (second_valid) {
    output = second;
    info = second_info;
  } else {
    output = default_config();
    info = StoredConfigInfo{};
    have_stored_ = false;
    last_sequence_ = 0;
    last_slot_ = 0;
    if (error != nullptr && error_size != 0) error[0] = '\0';
    return false;
  }

  have_stored_ = true;
  last_sequence_ = info.sequence;
  last_slot_ = info.slot;
  if (error != nullptr && error_size != 0) error[0] = '\0';
  return true;
}

bool ConfigStore::save(const DeviceConfig& config, StoredConfigInfo& info,
                       char* error, std::size_t error_size) {
  const std::uint32_t sequence = have_stored_ ? last_sequence_ + 1 : 1;
  const std::uint8_t slot = have_stored_ ? (last_slot_ == 0 ? 1 : 0) : 0;
  const std::uint32_t offset = slot == 0 ? kSlotAOffset : kSlotBOffset;

  slot_buffer_.fill(0xFF);
  const std::size_t payload_size = serialize_config(
      config, slot_buffer_.data() + kSlotHeaderSize,
      slot_buffer_.size() - kSlotHeaderSize);
  if (payload_size == 0) {
    return set_error(error, error_size, "config-serialize");
  }
  const std::uint32_t payload_crc =
      crc32(slot_buffer_.data() + kSlotHeaderSize, payload_size);

  write_u32(slot_buffer_.data(), kSlotMagic);
  write_u16(slot_buffer_.data() + 4, kSlotSchema);
  write_u16(slot_buffer_.data() + 6, 0);
  write_u32(slot_buffer_.data() + 8, sequence);
  write_u16(slot_buffer_.data() + 12, static_cast<std::uint16_t>(payload_size));
  write_u16(slot_buffer_.data() + 14, 0);
  write_u32(slot_buffer_.data() + 16, payload_crc);

  FlashWriteJob job{offset, slot_buffer_.data(), slot_buffer_.size()};
  if (flash_safe_execute(perform_flash_write, &job, 5000) != PICO_OK) {
    return set_error(error, error_size, "config-flash-write");
  }

  have_stored_ = true;
  last_sequence_ = sequence;
  last_slot_ = slot;
  info.from_flash = true;
  info.sequence = sequence;
  info.crc = payload_crc;
  info.size = static_cast<std::uint16_t>(payload_size);
  info.slot = slot;
  if (error != nullptr && error_size != 0) error[0] = '\0';
  return true;
}

bool ConfigStore::verify_last_write(char* error,
                                    std::size_t error_size) const {
  if (!have_stored_) {
    return set_error(error, error_size, "no-stored-config");
  }
  const std::uint32_t offset = last_slot_ == 0 ? kSlotAOffset : kSlotBOffset;
  DeviceConfig readback{};
  StoredConfigInfo readback_info{};
  if (!read_slot(offset, readback, readback_info, error, error_size)) {
    return set_error(error, error_size, "flash-readback-fail");
  }
  if (readback_info.sequence != last_sequence_) {
    return set_error(error, error_size, "flash-seq-mismatch");
  }
  if (error != nullptr && error_size != 0) error[0] = '\0';
  return true;
}

}  // namespace kohlrausch

