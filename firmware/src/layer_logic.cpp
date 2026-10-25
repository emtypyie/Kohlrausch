#include "layer_logic.hpp"
namespace kohlrausch {
uint8_t next_layer(uint8_t layer) { return layer >= 4 || layer < 1 ? 1 : static_cast<uint8_t>(layer + 1); }
}
