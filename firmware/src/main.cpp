#include "board_hardware.hpp"
#include "config_store.hpp"
#include "device_config.hpp"
#include "pico/stdlib.h"
#include "tusb.h"

#include <algorithm>
#include <array>
#include <cstddef>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>

#ifndef KOHLRAUSCH_FIRMWARE_VERSION
#define KOHLRAUSCH_FIRMWARE_VERSION "development"
#endif

namespace {

constexpr std::uint16_t kUsbVendorId = 0x2E8A;
constexpr std::uint16_t kUsbProductId = 0x10A1;
constexpr char kBoardId[] = "kohlrausch-rp2040-macropad";
constexpr char kHelloCommand[] = "KOHLRAUSCH/1 HELLO";
constexpr char kConfigPrefix[] = "KOHLRAUSCH/1 CONFIG ";
constexpr std::size_t kCommandCapacity = 256;
constexpr std::uint8_t kReportIdKeyboard = 1;
constexpr std::uint8_t kReportIdMouse = 2;
constexpr std::uint32_t kMacroHoldMs = 20;

std::array<bool, kohlrausch::kConfigKeyCount> held_keys{};
std::array<std::uint8_t, kohlrausch::kConfigKeyCount> held_modifiers{};
std::array<std::uint8_t, kohlrausch::kConfigKeyCount> held_keycodes{};
std::uint8_t active_layer = kohlrausch::kStartupLayer;

kohlrausch::DeviceConfig current_config = kohlrausch::default_config();
kohlrausch::StoredConfigInfo current_info{};
kohlrausch::ConfigStore config_store{};

std::array<std::uint8_t, kohlrausch::kConfigMaxBytes> config_staging{};
std::size_t config_expected_size = 0;
std::size_t config_received_size = 0;
std::uint32_t config_expected_crc = 0;
bool config_transfer_active = false;

const kohlrausch::Macro* active_macro = nullptr;
std::uint8_t active_macro_key = 0;
std::uint8_t macro_step_index = 0;
std::uint16_t macro_loops_done = 0;
bool macro_step_active = false;
std::uint32_t macro_release_at = 0;
std::uint32_t macro_next_at = 0;

struct MousePulse {
  std::uint32_t at_ms;
  std::uint8_t buttons;
  std::int8_t wheel;
};

std::array<MousePulse, 16> mouse_pulses{};
std::size_t mouse_head = 0;
std::size_t mouse_size = 0;

uint8_t const hid_report_descriptor[] = {
    TUD_HID_REPORT_DESC_KEYBOARD(HID_REPORT_ID(kReportIdKeyboard)),
    TUD_HID_REPORT_DESC_MOUSE(HID_REPORT_ID(kReportIdMouse))};

uint8_t const configuration_descriptor[] = {
    TUD_CONFIG_DESCRIPTOR(1, 3, 0,
                          TUD_CONFIG_DESC_LEN + TUD_CDC_DESC_LEN + TUD_HID_DESC_LEN,
                          TUSB_DESC_CONFIG_ATT_REMOTE_WAKEUP, 100),
    TUD_CDC_DESCRIPTOR(0, 4, 0x81, 8, 0x02, 0x82, 64),
    TUD_HID_DESCRIPTOR(2, 5, HID_ITF_PROTOCOL_NONE,
                       sizeof(hid_report_descriptor), 0x83, 16, 10)};

void respond(const char* text) {
  tud_cdc_write(text, std::strlen(text));
  tud_cdc_write_flush();
}

void clear_held_keys() {
  held_keys.fill(false);
  held_modifiers.fill(0);
  held_keycodes.fill(0);
}

void send_empty_keyboard() {
  const std::array<std::uint8_t, 6> keys{};
  tud_hid_keyboard_report(kReportIdKeyboard, 0, keys.data());
}

void send_held_report() {
  if (active_macro != nullptr) return;
  std::uint8_t modifiers = 0;
  std::array<std::uint8_t, 6> keys{};
  std::size_t count = 0;
  for (std::size_t key = 0; key < held_keys.size(); ++key) {
    if (!held_keys[key]) continue;
    modifiers |= held_modifiers[key];
    if (held_keycodes[key] != 0 && count < keys.size()) {
      keys[count++] = held_keycodes[key];
    }
  }
  tud_hid_keyboard_report(kReportIdKeyboard, modifiers, keys.data());
}

void enqueue_mouse_pulse(std::uint32_t at_ms, std::uint8_t buttons,
                         std::int8_t wheel) {
  if (mouse_size == mouse_pulses.size()) return;
  const auto tail = (mouse_head + mouse_size) % mouse_pulses.size();
  mouse_pulses[tail] = {at_ms, buttons, wheel};
  ++mouse_size;
}

void queue_mouse_action(std::uint8_t action, std::uint32_t now_ms) {
  switch (static_cast<kohlrausch::MouseAction>(action)) {
    case kohlrausch::MouseAction::LeftClick:
      enqueue_mouse_pulse(now_ms, 1, 0);
      enqueue_mouse_pulse(now_ms + 30, 0, 0);
      break;
    case kohlrausch::MouseAction::RightClick:
      enqueue_mouse_pulse(now_ms, 2, 0);
      enqueue_mouse_pulse(now_ms + 30, 0, 0);
      break;
    case kohlrausch::MouseAction::DoubleClick:
      enqueue_mouse_pulse(now_ms, 1, 0);
      enqueue_mouse_pulse(now_ms + 30, 0, 0);
      enqueue_mouse_pulse(now_ms + 100, 1, 0);
      enqueue_mouse_pulse(now_ms + 130, 0, 0);
      break;
    case kohlrausch::MouseAction::ScrollUp:
      enqueue_mouse_pulse(now_ms, 0, 1);
      enqueue_mouse_pulse(now_ms + 20, 0, 0);
      break;
    case kohlrausch::MouseAction::ScrollDown:
      enqueue_mouse_pulse(now_ms, 0, -1);
      enqueue_mouse_pulse(now_ms + 20, 0, 0);
      break;
    case kohlrausch::MouseAction::None:
      break;
  }
}

void update_mouse(std::uint32_t now_ms) {
  while (mouse_size != 0) {
    const auto& pulse = mouse_pulses[mouse_head];
    if (static_cast<std::int32_t>(now_ms - pulse.at_ms) < 0) break;
    tud_hid_mouse_report(kReportIdMouse, pulse.buttons, 0, 0, pulse.wheel, 0);
    mouse_head = (mouse_head + 1) % mouse_pulses.size();
    --mouse_size;
  }
}

std::uint32_t macro_step_delay(const kohlrausch::Macro& macro,
                               const kohlrausch::MacroStep& step) {
  return macro.timing == static_cast<std::uint8_t>(kohlrausch::MacroTiming::Recorded)
             ? step.delay_ms
             : macro.fixed_delay_ms;
}

bool macro_should_repeat(const kohlrausch::Macro& macro) {
  switch (static_cast<kohlrausch::MacroLoopMode>(macro.loop_mode)) {
    case kohlrausch::MacroLoopMode::Once:
      return false;
    case kohlrausch::MacroLoopMode::Count:
      return macro_loops_done < macro.loop_count;
    case kohlrausch::MacroLoopMode::UntilStop:
    case kohlrausch::MacroLoopMode::UntilAnyClick:
      return true;
  }
  return false;
}

void stop_macro(bool restore_held) {
  if (active_macro != nullptr && macro_step_active) {
    send_empty_keyboard();
  }
  active_macro = nullptr;
  macro_step_active = false;
  macro_step_index = 0;
  macro_loops_done = 0;
  if (restore_held) send_held_report();
}

void complete_macro_cycle(std::uint32_t now_ms) {
  if (active_macro == nullptr) return;
  ++macro_loops_done;
  if (!macro_should_repeat(*active_macro)) {
    stop_macro(true);
    return;
  }
  macro_step_index = 0;
  macro_next_at = now_ms;
}

void start_macro(std::uint8_t macro_index, std::uint8_t key_index,
                 std::uint32_t now_ms) {
  if (macro_index >= current_config.macro_count) return;
  if (current_config.macros[macro_index].step_count == 0) return;
  stop_macro(false);
  active_macro = &current_config.macros[macro_index];
  active_macro_key = key_index;
  macro_step_index = 0;
  macro_loops_done = 0;
  macro_step_active = false;
  macro_next_at = now_ms;
  send_empty_keyboard();
}

void update_macro(std::uint32_t now_ms) {
  if (active_macro == nullptr) return;
  if (macro_step_active) {
    if (static_cast<std::int32_t>(now_ms - macro_release_at) < 0) return;
    send_empty_keyboard();
    macro_step_active = false;
    const auto& step = active_macro->steps[macro_step_index];
    macro_next_at = now_ms + macro_step_delay(*active_macro, step);
    ++macro_step_index;
    if (macro_step_index >= active_macro->step_count) {
      complete_macro_cycle(now_ms);
    }
    return;
  }
  if (static_cast<std::int32_t>(now_ms - macro_next_at) < 0) return;
  const auto& step = active_macro->steps[macro_step_index];
  if (step.kind == static_cast<std::uint8_t>(kohlrausch::BindingKind::Chord)) {
    const std::array<std::uint8_t, 6> keys{step.keycode, 0, 0, 0, 0, 0};
    tud_hid_keyboard_report(kReportIdKeyboard, step.modifiers, keys.data());
    macro_step_active = true;
    macro_release_at = now_ms + kMacroHoldMs;
    return;
  }
  if (step.kind == static_cast<std::uint8_t>(kohlrausch::BindingKind::Mouse)) {
    queue_mouse_action(step.mouse_action, now_ms);
    macro_next_at = now_ms + macro_step_delay(*active_macro, step);
    ++macro_step_index;
    if (macro_step_index >= active_macro->step_count) {
      complete_macro_cycle(now_ms);
    }
  }
}

void handle_key_event(const kohlrausch::ports::KeyEvent& event,
                      std::uint32_t now_ms) {
  const auto index = static_cast<std::size_t>(event.key);
  if (index >= held_keys.size()) return;
  if (event.pressed) {
    if (active_macro != nullptr) {
      if (index == active_macro_key) {
        stop_macro(true);
        return;
      }
      if (active_macro->loop_mode ==
          static_cast<std::uint8_t>(kohlrausch::MacroLoopMode::UntilAnyClick)) {
        stop_macro(true);
      }
    }
    const auto& binding =
        current_config.layers[active_layer - 1].keys[index];
    switch (static_cast<kohlrausch::BindingKind>(binding.kind)) {
      case kohlrausch::BindingKind::Chord:
        held_keys[index] = true;
        held_modifiers[index] = binding.modifiers;
        held_keycodes[index] = binding.keycode;
        send_held_report();
        break;
      case kohlrausch::BindingKind::Mouse:
        queue_mouse_action(binding.mouse_action, now_ms);
        break;
      case kohlrausch::BindingKind::Macro:
        start_macro(binding.macro_index, static_cast<std::uint8_t>(index), now_ms);
        break;
      case kohlrausch::BindingKind::None:
        break;
    }
  } else if (held_keys[index]) {
    held_keys[index] = false;
    held_modifiers[index] = 0;
    held_keycodes[index] = 0;
    send_held_report();
  }
}

void skip_spaces(const char*& cursor) {
  while (*cursor == ' ') ++cursor;
}

bool parse_number(const char*& cursor, int base, std::uint32_t& value) {
  skip_spaces(cursor);
  if (*cursor == '\0') return false;
  char* end = nullptr;
  const auto parsed = std::strtoul(cursor, &end, base);
  if (end == cursor) return false;
  value = static_cast<std::uint32_t>(parsed);
  cursor = end;
  return true;
}

bool at_end(const char* cursor) {
  skip_spaces(cursor);
  return *cursor == '\0';
}

int hex_digit(char value) {
  if (value >= '0' && value <= '9') return value - '0';
  if (value >= 'a' && value <= 'f') return value - 'a' + 10;
  if (value >= 'A' && value <= 'F') return value - 'A' + 10;
  return -1;
}

void config_error(const char* code) {
  char response[96]{};
  std::snprintf(response, sizeof(response),
                "KOHLRAUSCH/1 CONFIG ERROR %s\r\n", code);
  respond(response);
}

void handle_config_command(const char* command) {
  if (std::strncmp(command, "BEGIN ", 6) == 0) {
    const char* cursor = command + 6;
    std::uint32_t size = 0;
    std::uint32_t crc = 0;
    if (!parse_number(cursor, 10, size) || !parse_number(cursor, 16, crc) ||
        !at_end(cursor) || size == 0 || size > kohlrausch::kConfigMaxBytes) {
      config_error("invalid-begin");
      return;
    }
    config_expected_size = size;
    config_received_size = 0;
    config_expected_crc = crc;
    config_transfer_active = true;
    char response[64]{};
    std::snprintf(response, sizeof(response),
                  "KOHLRAUSCH/1 CONFIG READY %u\r\n", static_cast<unsigned>(size));
    respond(response);
    return;
  }

  if (std::strncmp(command, "DATA ", 5) == 0) {
    if (!config_transfer_active) {
      config_error("no-transfer");
      return;
    }
    const char* cursor = command + 5;
    std::uint32_t offset = 0;
    if (!parse_number(cursor, 10, offset)) {
      config_error("invalid-offset");
      return;
    }
    skip_spaces(cursor);
    const std::size_t hex_length = std::strlen(cursor);
    if (hex_length == 0 || (hex_length % 2) != 0 ||
        offset != config_received_size ||
        offset + hex_length / 2 > config_expected_size) {
      config_error("invalid-data");
      return;
    }
    for (std::size_t index = 0; index < hex_length; index += 2) {
      const int high = hex_digit(cursor[index]);
      const int low = hex_digit(cursor[index + 1]);
      if (high < 0 || low < 0) {
        config_error("invalid-hex");
        return;
      }
      config_staging[offset + index / 2] =
          static_cast<std::uint8_t>((high << 4) | low);
    }
    config_received_size += hex_length / 2;
    char response[64]{};
    std::snprintf(response, sizeof(response),
                  "KOHLRAUSCH/1 CONFIG OK %u\r\n",
                  static_cast<unsigned>(config_received_size));
    respond(response);
    return;
  }

  if (std::strncmp(command, "COMMIT ", 7) == 0) {
    const char* cursor = command + 7;
    std::uint32_t size = 0;
    std::uint32_t crc = 0;
    if (!config_transfer_active) {
      config_error("no-transfer");
      return;
    }
    if (!parse_number(cursor, 10, size) || !parse_number(cursor, 16, crc) ||
        !at_end(cursor) || size != config_expected_size ||
        config_received_size != config_expected_size || crc != config_expected_crc) {
      config_error("invalid-commit");
      return;
    }
    if (kohlrausch::crc32(config_staging.data(), config_expected_size) != crc) {
      config_error("crc-mismatch");
      return;
    }
    kohlrausch::DeviceConfig candidate{};
    char parse_error[32]{};
    if (!kohlrausch::parse_config(config_staging.data(), config_expected_size,
                                  candidate, parse_error, sizeof(parse_error))) {
      config_error(parse_error);
      return;
    }
    kohlrausch::StoredConfigInfo new_info{};
    char save_error[32]{};
    if (!config_store.save(candidate, new_info, save_error, sizeof(save_error))) {
      config_error(save_error);
      return;
    }
    char verify_error[32]{};
    if (!config_store.verify_last_write(verify_error, sizeof(verify_error))) {
      config_error(verify_error);
      return;
    }
    stop_macro(false);
    clear_held_keys();
    send_empty_keyboard();
    current_config = candidate;
    current_info = new_info;
    config_transfer_active = false;
    char response[96]{};
    std::snprintf(response, sizeof(response),
                  "KOHLRAUSCH/1 CONFIG SAVED %08lX\r\n",
                  static_cast<unsigned long>(crc));
    respond(response);
    return;
  }

  if (std::strcmp(command, "STATUS") == 0) {
    char response[160]{};
    std::snprintf(response, sizeof(response),
                  "KOHLRAUSCH/1 CONFIG STATUS source=%s version=1 sequence=%lu "
                  "bytes=%u crc=%08lX\r\n",
                  current_info.from_flash ? "flash" : "default",
                  static_cast<unsigned long>(current_info.sequence),
                  static_cast<unsigned>(current_info.size),
                  static_cast<unsigned long>(current_info.crc));
    respond(response);
    return;
  }

  if (std::strcmp(command, "RESET") == 0) {
    const auto fallback = kohlrausch::default_config();
    kohlrausch::StoredConfigInfo new_info{};
    char save_error[32]{};
    if (!config_store.save(fallback, new_info, save_error, sizeof(save_error))) {
      config_error(save_error);
      return;
    }
    char verify_error[32]{};
    if (!config_store.verify_last_write(verify_error, sizeof(verify_error))) {
      config_error(verify_error);
      return;
    }
    stop_macro(false);
    clear_held_keys();
    send_empty_keyboard();
    current_config = fallback;
    current_info = new_info;
    config_transfer_active = false;
    respond("KOHLRAUSCH/1 CONFIG RESET\r\n");
    return;
  }

  config_error("unsupported-command");
}

void handle_command(const char* command) {
  if (std::strcmp(command, kHelloCommand) == 0) {
    char response[96]{};
    std::snprintf(response, sizeof(response), "KOHLRAUSCH/1 DEVICE %s %s\r\n",
                  kBoardId, KOHLRAUSCH_FIRMWARE_VERSION);
    respond(response);
  } else if (std::strcmp(command, "KOHLRAUSCH/1 STATUS") == 0) {
    char response[64]{};
    std::snprintf(response, sizeof(response), "KOHLRAUSCH/1 STATUS layer=%u\r\n",
                  static_cast<unsigned>(active_layer));
    respond(response);
  } else if (std::strncmp(command, kConfigPrefix,
                          sizeof(kConfigPrefix) - 1) == 0) {
    handle_config_command(command + sizeof(kConfigPrefix) - 1);
  } else {
    respond("KOHLRAUSCH/1 ERROR unsupported-command\r\n");
  }
}

void poll_commands() {
  static std::array<char, kCommandCapacity> command{};
  static std::size_t command_size = 0;
  static bool discarding = false;
  while (tud_cdc_available()) {
    char character = 0;
    tud_cdc_read(&character, 1);
    if (character == '\r') continue;
    if (character == '\n') {
      if (!discarding && command_size != 0) {
        command[command_size] = '\0';
        handle_command(command.data());
      } else if (discarding) {
        respond("KOHLRAUSCH/1 ERROR command-too-long\r\n");
      }
      command_size = 0;
      discarding = false;
      continue;
    }
    if (discarding) continue;
    if (command_size + 1 >= command.size()) {
      command_size = 0;
      discarding = true;
    } else {
      command[command_size++] = character;
    }
  }
}

}  // namespace

extern "C" {

uint8_t const* tud_hid_descriptor_report_cb(uint8_t instance) {
  (void)instance;
  return hid_report_descriptor;
}

uint16_t const* tud_descriptor_string_cb(uint8_t index, uint16_t langid) {
  (void)langid;
  static uint16_t descriptor[32];
  if (index == 0) {
    descriptor[0] = (TUSB_DESC_STRING << 8) | 4;
    descriptor[1] = 0x0409;
    return descriptor;
  }
  const char* value = "Kohlrausch";
  if (index == 1) value = "Visrodeck Instruments";
  if (index == 2) value = "Kohlrausch RP2040";
  if (index == 3) value = "0001";
  std::size_t length = std::strlen(value);
  if (length > 31) length = 31;
  descriptor[0] = static_cast<std::uint16_t>((TUSB_DESC_STRING << 8) | (2 * length + 2));
  for (std::size_t i = 0; i < length; ++i) descriptor[i + 1] = value[i];
  return descriptor;
}

uint8_t const* tud_descriptor_device_cb(void) {
  static tusb_desc_device_t const descriptor = {
      .bLength = sizeof(tusb_desc_device_t),
      .bDescriptorType = TUSB_DESC_DEVICE,
      .bcdUSB = 0x0200,
      .bDeviceClass = TUSB_CLASS_MISC,
      .bDeviceSubClass = MISC_SUBCLASS_COMMON,
      .bDeviceProtocol = MISC_PROTOCOL_IAD,
      .bMaxPacketSize0 = CFG_TUD_ENDPOINT0_SIZE,
      .idVendor = kUsbVendorId,
      .idProduct = kUsbProductId,
      .bcdDevice = 0x0100,
      .iManufacturer = 1,
      .iProduct = 2,
      .iSerialNumber = 3,
      .bNumConfigurations = 1};
  return reinterpret_cast<uint8_t const*>(&descriptor);
}

uint8_t const* tud_descriptor_configuration_cb(uint8_t index) {
  (void)index;
  return configuration_descriptor;
}

void tud_hid_set_report_cb(uint8_t instance, uint8_t report_id,
                           hid_report_type_t report_type,
                           uint8_t const* buffer, uint16_t bufsize) {
  (void)instance; (void)report_id; (void)report_type; (void)buffer; (void)bufsize;
}

uint16_t tud_hid_get_report_cb(uint8_t instance, uint8_t report_id,
                               hid_report_type_t report_type,
                               uint8_t* buffer, uint16_t reqlen) {
  (void)instance; (void)report_id; (void)report_type; (void)buffer; (void)reqlen;
  return 0;
}

}  // extern "C"

int main() {
  stdio_init_all();
  tusb_init();

  char config_error_text[32]{};
  config_store.load(current_config, current_info, config_error_text,
                    sizeof(config_error_text));

  kohlrausch::board::MatrixKeyScanner scanner;
  kohlrausch::board::LayerSwitch layer_switch;
#if KOHLRAUSCH_ENABLE_OLED
  kohlrausch::board::Ssd1306Display display;
  display.initialize();
#endif
  kohlrausch::FirmwareRuntime runtime;
  runtime.begin(layer_switch.is_pressed(), to_ms_since_boot(get_absolute_time()));
#if KOHLRAUSCH_ENABLE_OLED
  display.render(runtime.update(layer_switch.is_pressed(),
                                to_ms_since_boot(get_absolute_time())).oled_frame);
#else
  runtime.update(layer_switch.is_pressed(), to_ms_since_boot(get_absolute_time()));
#endif

  while (true) {
    tud_task();
    poll_commands();
    const auto now = to_ms_since_boot(get_absolute_time());
    scanner.poll(now);
    update_mouse(now);
    update_macro(now);
    auto update = runtime.update(layer_switch.is_pressed(), now);
    if (update.layer_changed) {
      stop_macro(false);
      clear_held_keys();
      send_empty_keyboard();
    }
    active_layer = update.active_layer;
#if KOHLRAUSCH_ENABLE_OLED
    if (update.oled_frame_changed) display.render(update.oled_frame);
#endif

    kohlrausch::ports::KeyEvent event{};
    while (scanner.next_event(event)) {
      handle_key_event(event, now);
    }
    sleep_ms(5);
  }
}
