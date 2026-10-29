#pragma once

#include "../include/hardware_ports.hpp"

#include <array>
#include <cstdint>

namespace kohlrausch::board {

class MatrixKeyScanner final : public ports::KeyScanner {
 public:
  MatrixKeyScanner();
  void poll(std::uint32_t now_ms) override;
  bool next_event(ports::KeyEvent& event) override;

 private:
  struct KeyState {
    std::uint32_t candidate_since_ms = 0;
    bool candidate_pressed = false;
    bool stable_pressed = false;
  };

  bool enqueue(ports::KeyEvent event);

  std::array<std::array<KeyState, 3>, 3> keys_{};
  std::array<ports::KeyEvent, 16> events_{};
  std::size_t event_head_ = 0;
  std::size_t event_size_ = 0;
};

class LayerSwitch final : public ports::LyrInput {
 public:
  LayerSwitch();
  bool is_pressed() override;
};

#if KOHLRAUSCH_ENABLE_OLED
class Ssd1306Display final : public ports::OledOutput {
 public:
  Ssd1306Display();
  bool initialize();
  void render(const OledFrame& frame) override;
  bool ready() const noexcept;

 private:
  bool send_command(std::uint8_t command);
  bool send_commands(const std::uint8_t* commands, std::size_t size);
  void clear() noexcept;
  void set_pixel(std::int16_t x, std::int16_t y) noexcept;
  void draw_text(const char* text, std::int16_t x, std::int16_t y,
                 std::uint8_t scale) noexcept;
  void draw_centered(const char* text, std::int16_t y,
                     std::uint8_t scale) noexcept;
  bool flush();

  std::array<std::uint8_t, 128 * 32 / 8> framebuffer_{};
  bool ready_ = false;
};
#endif

}  // namespace kohlrausch::board
