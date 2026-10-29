cmake_minimum_required(VERSION 3.20)
if(NOT DEFINED ENV{PICO_SDK_PATH})
  message(FATAL_ERROR "Set PICO_SDK_PATH to a Raspberry Pi Pico SDK checkout before configuring firmware.")
endif()
include($ENV{PICO_SDK_PATH}/external/pico_sdk_import.cmake)
project(kohlrausch_firmware VERSION 0.1.0 LANGUAGES C CXX ASM)

set(KOHLRAUSCH_BOARD_FILE
  "${CMAKE_CURRENT_SOURCE_DIR}/../packages/board-definitions/boards/kohlrausch-rp2040.json")
file(READ "${KOHLRAUSCH_BOARD_FILE}" KOHLRAUSCH_BOARD_JSON)
foreach(index RANGE 0 2)
  string(JSON KOHLRAUSCH_ROW_GPIO_${index} GET
    "${KOHLRAUSCH_BOARD_JSON}" matrix rowGpios ${index})
  string(JSON KOHLRAUSCH_COLUMN_GPIO_${index} GET
    "${KOHLRAUSCH_BOARD_JSON}" matrix columnGpios ${index})
endforeach()
string(JSON KOHLRAUSCH_LYR_GPIO GET
  "${KOHLRAUSCH_BOARD_JSON}" lyrSwitch gpio)
option(KOHLRAUSCH_ENABLE_OLED
  "Build the OLED driver after controller, geometry, address and bus settings are verified" OFF)
if(KOHLRAUSCH_ENABLE_OLED)
  string(JSON KOHLRAUSCH_OLED_STATUS GET "${KOHLRAUSCH_BOARD_JSON}" oled status)
  string(JSON KOHLRAUSCH_OLED_CONTROLLER GET "${KOHLRAUSCH_BOARD_JSON}" oled controller)
  string(JSON KOHLRAUSCH_OLED_RESOLUTION GET "${KOHLRAUSCH_BOARD_JSON}" oled resolution)
  string(JSON KOHLRAUSCH_OLED_ADDRESS GET "${KOHLRAUSCH_BOARD_JSON}" oled i2cAddress)
  string(JSON KOHLRAUSCH_OLED_I2C_BUS GET "${KOHLRAUSCH_BOARD_JSON}" oled i2cBus)
  string(JSON KOHLRAUSCH_OLED_FREQUENCY_HZ GET "${KOHLRAUSCH_BOARD_JSON}" oled frequencyHz)
  string(JSON KOHLRAUSCH_OLED_SDA_GPIO GET "${KOHLRAUSCH_BOARD_JSON}" oled sdaGpio)
  string(JSON KOHLRAUSCH_OLED_SCL_GPIO GET "${KOHLRAUSCH_BOARD_JSON}" oled sclGpio)
  if(KOHLRAUSCH_OLED_STATUS STREQUAL "pending-verification"
      OR KOHLRAUSCH_OLED_CONTROLLER STREQUAL "null"
      OR KOHLRAUSCH_OLED_RESOLUTION STREQUAL "null"
      OR KOHLRAUSCH_OLED_ADDRESS STREQUAL "null"
      OR KOHLRAUSCH_OLED_I2C_BUS STREQUAL "null"
      OR KOHLRAUSCH_OLED_FREQUENCY_HZ STREQUAL "null")
    message(FATAL_ERROR "OLED support requires verified controller, resolution, address, bus, and frequency in board metadata.")
  endif()
  if(NOT KOHLRAUSCH_OLED_CONTROLLER STREQUAL "SSD1306")
    message(FATAL_ERROR "The selected open-source OLED driver supports SSD1306 only.")
  endif()
else()
  set(KOHLRAUSCH_OLED_SDA_GPIO 0)
  set(KOHLRAUSCH_OLED_SCL_GPIO 0)
  set(KOHLRAUSCH_OLED_I2C_BUS 0)
  set(KOHLRAUSCH_OLED_FREQUENCY_HZ 0)
  set(KOHLRAUSCH_OLED_ADDRESS 0)
endif()
file(MAKE_DIRECTORY "${CMAKE_CURRENT_BINARY_DIR}/generated")
configure_file(
  include/board_config.hpp.in
  "${CMAKE_CURRENT_BINARY_DIR}/generated/board_config.hpp"
  @ONLY
)

pico_sdk_init()
if(NOT TARGET tinyusb_device OR NOT TARGET tinyusb_board)
  message(FATAL_ERROR "Pico SDK TinyUSB device targets are unavailable. Check that the SDK TinyUSB submodule is initialized.")
endif()
add_executable(kohlrausch_firmware
  src/main.cpp
  src/layer_logic.cpp
  src/runtime_logic.cpp
  src/board_hardware.cpp
  src/device_config.cpp
  src/config_store.cpp
)
target_compile_features(kohlrausch_firmware PRIVATE cxx_std_20)
target_include_directories(kohlrausch_firmware PRIVATE include src)
target_include_directories(kohlrausch_firmware PRIVATE "${CMAKE_CURRENT_BINARY_DIR}/generated")
target_compile_definitions(kohlrausch_firmware PRIVATE
  KOHLRAUSCH_FIRMWARE_VERSION="${PROJECT_VERSION}"
  KOHLRAUSCH_ENABLE_OLED=$<BOOL:${KOHLRAUSCH_ENABLE_OLED}>
)
target_link_libraries(kohlrausch_firmware
  pico_stdlib
  hardware_flash
  pico_flash
  tinyusb_device
  tinyusb_board
)
if(KOHLRAUSCH_ENABLE_OLED)
  target_link_libraries(kohlrausch_firmware hardware_i2c)
endif()
pico_enable_stdio_usb(kohlrausch_firmware 0)
pico_enable_stdio_uart(kohlrausch_firmware 0)

# Reserve final 16 KB of flash (two 8 KB slots) from the application link region
pico_add_linker_script_override_path(kohlrausch_firmware
  "${CMAKE_CURRENT_SOURCE_DIR}/linker" FILES pico_flash_region.ld)
target_link_options(kohlrausch_firmware PRIVATE
  "-Wl,-Map=${CMAKE_CURRENT_BINARY_DIR}/kohlrausch_firmware.elf.map")

pico_add_extra_outputs(kohlrausch_firmware)

