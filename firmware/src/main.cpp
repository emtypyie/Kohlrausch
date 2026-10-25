#include "pico/stdlib.h"
#include "layer_logic.hpp"
// Firmware entry is intentionally inactive until schematic-verified GPIO and transport drivers exist.
int main() {
  stdio_init_all();
  const uint8_t startup_layer = 1;
  (void)startup_layer;
  while (true) { tight_loop_contents(); }
}
