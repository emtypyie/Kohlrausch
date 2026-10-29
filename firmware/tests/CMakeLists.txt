cmake_minimum_required(VERSION 3.20)
project(kohlrausch_layer_logic LANGUAGES CXX)
add_executable(layer_logic_test
  ../src/layer_logic.cpp
  ../src/runtime_logic.cpp
  layer_logic_test.cpp
)
target_compile_features(layer_logic_test PRIVATE cxx_std_17)

add_executable(device_config_test
  ../src/device_config.cpp
  device_config_test.cpp
)
target_compile_features(device_config_test PRIVATE cxx_std_17)

enable_testing()
add_test(NAME layer_logic_test COMMAND layer_logic_test)
add_test(NAME device_config_test COMMAND device_config_test)
