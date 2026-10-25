cmake_minimum_required(VERSION 3.20)
project(kohlrausch_layer_logic LANGUAGES CXX)
add_executable(layer_logic_test
  ../src/layer_logic.cpp
  ../src/runtime_logic.cpp
  layer_logic_test.cpp
)
target_compile_features(layer_logic_test PRIVATE cxx_std_17)
enable_testing()
add_test(NAME layer_logic_test COMMAND layer_logic_test)
