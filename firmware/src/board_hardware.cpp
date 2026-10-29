#include "board_hardware.hpp"

#include "board_config.hpp"
#include "hardware/gpio.h"
#include "pico/stdlib.h"
#if KOHLRAUSCH_ENABLE_OLED
#include "hardware/i2c.h"
#endif

#include <algorithm>
#include <array>
#include <cstring>

namespace kohlrausch::board {
namespace {

#if KOHLRAUSCH_ENABLE_OLED
i2c_inst_t* display_bus() {
  return kOledI2cBus == 0 ? i2c0 : i2c1;
}
#endif

#if KOHLRAUSCH_ENABLE_OLED
// Five-column glyphs, least-significant bit at the top of each column.
std::array<std::uint8_t, 5> glyph(char value) {
  switch (value) {
    case 'A': return {0x7e, 0x11, 0x11, 0x11, 0x7e};
    case 'B': return {0x7f, 0x49, 0x49, 0x49, 0x36};
    case 'C': return {0x3e, 0x41, 0x41, 0x41, 0x22};
    case 'D': return {0x7f, 0x41, 0x41, 0x22, 0x1c};
    case 'E': return {0x7f, 0x49, 0x49, 0x49, 0x41};
    case 'F': return {0x7f, 0x09, 0x09, 0x09, 0x01};
    case 'G': return {0x3e, 0x41, 0x49, 0x49, 0x7a};
    case 'H': return {0x7f, 0x08, 0x08, 0x08, 0x7f};
    case 'I': return {0x00, 0x41, 0x7f, 0x41, 0x00};
    case 'K': return {0x7f, 0x08, 0x14, 0x22, 0x41};
    case 'L': return {0x7f, 0x40, 0x40, 0x40, 0x40};
    case 'O': return {0x3e, 0x41, 0x41, 0x41, 0x3e};
    case 'R': return {0x7f, 0x09, 0x19, 0x29, 0x46};
    case 'S': return {0x46, 0x49, 0x49, 0x49, 0x31};
    case 'U': return {0x3f, 0x40, 0x40, 0x40, 0x3f};
    case '1': return {0x00, 0x42, 0x7f, 0x40, 0x00};
    case '2': return {0x62, 0x51, 0x49, 0x49, 0x46};
    case '3': return {0x22, 0x41, 0x49, 0x49, 0x36};
    case '4': return {0x18, 0x14, 0x12, 0x7f, 0x10};
    case ' ': return {0, 0, 0, 0, 0};
    default: return {0, 0, 0, 0, 0};
  }
}
#endif

}  // namespace

MatrixKeyScanner::MatrixKeyScanner() {
  for (const auto pin : kMatrixRows) {
    gpio_init(pin);
    gpio_set_dir(pin, GPIO_IN);
    gpio_pull_down(pin);
  }
  for (const auto pin : kMatrixColumns) {
    gpio_init(pin);
    gpio_set_dir(pin, GPIO_OUT);
    gpio_put(pin, 0);
  }
}

bool MatrixKeyScanner::enqueue(ports::KeyEvent event) {
  if (event_size_ == events_.size()) return false;
  const auto tail = (event_head_ + event_size_) % events_.size();
  events_[tail] = event;
  ++event_size_;
  return true;
}

void MatrixKeyScanner::poll(std::uint32_t now_ms) {
  // Diodes point column -> row: drive one column high and sample pulled-down rows.
  for (std::size_t column = 0; column < kMatrixColumns.size(); ++column) {
    for (const auto pin : kMatrixColumns) gpio_put(pin, 0);
    gpio_put(kMatrixColumns[column], 1);
    sleep_us(40);
    for (std::size_t row = 0; row < kMatrixRows.size(); ++row) {
      auto& key = keys_[row][column];
      const bool pressed = gpio_get(kMatrixRows[row]) != 0;
      if (pressed != key.candidate_pressed) {
        key.candidate_pressed = pressed;
        key.candidate_since_ms = now_ms;
      } else if (pressed != key.stable_pressed &&
                 static_cast<std::uint32_t>(now_ms - key.candidate_since_ms) >= 20) {
        key.stable_pressed = pressed;
        const auto index = row * kMatrixColumns.size() + column;
        enqueue({static_cast<ports::KeyId>(index), pressed});
      }
    }
  }
  for (const auto pin : kMatrixColumns) gpio_put(pin, 0);
}

bool MatrixKeyScanner::next_event(ports::KeyEvent& event) {
  if (event_size_ == 0) return false;
  event = events_[event_head_];
  event_head_ = (event_head_ + 1) % events_.size();
  --event_size_;
  return true;
}

LayerSwitch::LayerSwitch() {
  gpio_init(kLyrSwitchGpio);
  gpio_set_dir(kLyrSwitchGpio, GPIO_IN);
  gpio_pull_up(kLyrSwitchGpio);
}

bool LayerSwitch::is_pressed() { return gpio_get(kLyrSwitchGpio) == 0; }

#if KOHLRAUSCH_ENABLE_OLED
Ssd1306Display::Ssd1306Display() = default;

bool Ssd1306Display::send_command(std::uint8_t command) {
  const std::array<std::uint8_t, 2> packet{0x00, command};
  return i2c_write_blocking(display_bus(), kOledAddress, packet.data(), packet.size(), false) ==
         static_cast<int>(packet.size());
}

bool Ssd1306Display::send_commands(const std::uint8_t* commands, std::size_t size) {
  for (std::size_t i = 0; i < size; ++i) {
    if (!send_command(commands[i])) return false;
  }
  return true;
}

bool Ssd1306Display::initialize() {
  i2c_init(display_bus(), kOledFrequencyHz);
  gpio_set_function(kOledSdaGpio, GPIO_FUNC_I2C);
  gpio_set_function(kOledSclGpio, GPIO_FUNC_I2C);
  gpio_pull_up(kOledSdaGpio);
  gpio_pull_up(kOledSclGpio);
  constexpr std::array<std::uint8_t, 26> init = {
      0xAE, 0xD5, 0x80, 0xA8, 0x1F, 0xD3, 0x00, 0x40, 0x8D, 0x14,
      0x20, 0x00, 0xA1, 0xC8, 0xDA, 0x02, 0x81, 0x8F, 0xD9, 0xF1,
      0xDB, 0x40, 0xA4, 0xA6, 0xAF, 0x2E};
  ready_ = send_commands(init.data(), init.size());
  if (ready_) {
    clear();
    ready_ = flush();
  }
  return ready_;
}

void Ssd1306Display::clear() noexcept { framebuffer_.fill(0); }

void Ssd1306Display::set_pixel(std::int16_t x, std::int16_t y) noexcept {
  if (x < 0 || x >= kOledWidth || y < 0 || y >= kOledHeight) return;
  const auto position = static_cast<std::size_t>(x) +
                        static_cast<std::size_t>(y / 8) * kOledWidth;
  framebuffer_[position] |= static_cast<std::uint8_t>(1U << (y % 8));
}

void Ssd1306Display::draw_text(const char* text, std::int16_t x, std::int16_t y,
                               std::uint8_t scale) noexcept {
  for (const char* cursor = text; *cursor; ++cursor) {
    const auto columns = glyph(*cursor);
    for (std::size_t column = 0; column < columns.size(); ++column) {
      for (std::uint8_t bit = 0; bit < 7; ++bit) {
        if ((columns[column] & (1U << bit)) == 0) continue;
        for (std::uint8_t dx = 0; dx < scale; ++dx)
          for (std::uint8_t dy = 0; dy < scale; ++dy)
            set_pixel(x + static_cast<std::int16_t>(column * scale + dx),
                      y + static_cast<std::int16_t>(bit * scale + dy));
      }
    }
    x += static_cast<std::int16_t>(6 * scale);
  }
}

void Ssd1306Display::draw_centered(const char* text, std::int16_t y,
                                   std::uint8_t scale) noexcept {
  const auto width = static_cast<std::int16_t>(std::strlen(text) * 6 * scale - scale);
  draw_text(text, (static_cast<std::int16_t>(kOledWidth) - width) / 2, y, scale);
}

bool Ssd1306Display::flush() {
  constexpr std::array<std::uint8_t, 6> window = {0x00, 0x21, 0, 127, 0x22, 0};
  if (!send_commands(window.data() + 1, 1) || !send_command(0) ||
      !send_command(127) || !send_commands(window.data() + 4, 1) ||
      !send_command(0) || !send_command(3)) return false;
  for (std::size_t offset = 0; offset < framebuffer_.size(); offset += 16) {
    std::array<std::uint8_t, 17> packet{};
    packet[0] = 0x40;
    std::copy_n(framebuffer_.begin() + offset, 16, packet.begin() + 1);
    if (i2c_write_blocking(display_bus(), kOledAddress, packet.data(), packet.size(), false) !=
        static_cast<int>(packet.size())) return false;
  }
  return true;
}

void Ssd1306Display::render(const OledFrame& frame) {
  if (!ready_) return;
  clear();
  if (frame.screen == OledScreen::LayerSelector) {
    draw_centered("LAYER", 2, 1);
    const char layer[] = {static_cast<char>('0' + frame.active_layer), '\0'};
    draw_centered(layer, 13, 2);
  } else {
    draw_centered("KOHLRAUSCH", 12, 1);
  }
  flush();
}

bool Ssd1306Display::ready() const noexcept { return ready_; }
#endif

}  // namespace kohlrausch::board
