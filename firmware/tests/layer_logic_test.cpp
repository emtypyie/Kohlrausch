#include "../src/layer_logic.hpp"
#include <cassert>
int main() {
  assert(kohlrausch::next_layer(1) == 2);
  assert(kohlrausch::next_layer(2) == 3);
  assert(kohlrausch::next_layer(3) == 4);
  assert(kohlrausch::next_layer(4) == 1);
  assert(kohlrausch::next_layer(0) == 1);
}
