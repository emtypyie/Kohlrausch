# CMake generated Testfile for 
# Source directory: D:/EMTYPYIE/Kohlrausch/firmware/tests
# Build directory: D:/EMTYPYIE/Kohlrausch/work/firmware-host-build
# 
# This file includes the relevant testing commands required for 
# testing this directory and lists subdirectories to be tested as well.
if(CTEST_CONFIGURATION_TYPE MATCHES "^([Dd][Ee][Bb][Uu][Gg])$")
  add_test([=[layer_logic_test]=] "D:/EMTYPYIE/Kohlrausch/work/firmware-host-build/Debug/layer_logic_test.exe")
  set_tests_properties([=[layer_logic_test]=] PROPERTIES  _BACKTRACE_TRIPLES "D:/EMTYPYIE/Kohlrausch/firmware/tests/CMakeLists.txt;6;add_test;D:/EMTYPYIE/Kohlrausch/firmware/tests/CMakeLists.txt;0;")
elseif(CTEST_CONFIGURATION_TYPE MATCHES "^([Rr][Ee][Ll][Ee][Aa][Ss][Ee])$")
  add_test([=[layer_logic_test]=] "D:/EMTYPYIE/Kohlrausch/work/firmware-host-build/Release/layer_logic_test.exe")
  set_tests_properties([=[layer_logic_test]=] PROPERTIES  _BACKTRACE_TRIPLES "D:/EMTYPYIE/Kohlrausch/firmware/tests/CMakeLists.txt;6;add_test;D:/EMTYPYIE/Kohlrausch/firmware/tests/CMakeLists.txt;0;")
elseif(CTEST_CONFIGURATION_TYPE MATCHES "^([Mm][Ii][Nn][Ss][Ii][Zz][Ee][Rr][Ee][Ll])$")
  add_test([=[layer_logic_test]=] "D:/EMTYPYIE/Kohlrausch/work/firmware-host-build/MinSizeRel/layer_logic_test.exe")
  set_tests_properties([=[layer_logic_test]=] PROPERTIES  _BACKTRACE_TRIPLES "D:/EMTYPYIE/Kohlrausch/firmware/tests/CMakeLists.txt;6;add_test;D:/EMTYPYIE/Kohlrausch/firmware/tests/CMakeLists.txt;0;")
elseif(CTEST_CONFIGURATION_TYPE MATCHES "^([Rr][Ee][Ll][Ww][Ii][Tt][Hh][Dd][Ee][Bb][Ii][Nn][Ff][Oo])$")
  add_test([=[layer_logic_test]=] "D:/EMTYPYIE/Kohlrausch/work/firmware-host-build/RelWithDebInfo/layer_logic_test.exe")
  set_tests_properties([=[layer_logic_test]=] PROPERTIES  _BACKTRACE_TRIPLES "D:/EMTYPYIE/Kohlrausch/firmware/tests/CMakeLists.txt;6;add_test;D:/EMTYPYIE/Kohlrausch/firmware/tests/CMakeLists.txt;0;")
else()
  add_test([=[layer_logic_test]=] NOT_AVAILABLE)
endif()
