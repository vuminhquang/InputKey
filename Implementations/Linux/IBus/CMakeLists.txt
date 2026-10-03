cmake_minimum_required(VERSION 3.16)
project(inputkey-ibus VERSION 5.2.6 LANGUAGES C)

include(GNUInstallDirs)
find_package(PkgConfig REQUIRED)
pkg_check_modules(IBUS REQUIRED ibus-1.0)

set(INPUTKEY_NATIVE_LIB_DIR "" CACHE PATH "Directory containing the Rust InputKey native library")
find_library(INPUTKEY_NATIVE_LIB NAMES inputkey_cabi REQUIRED
    HINTS "${INPUTKEY_NATIVE_LIB_DIR}" "${CMAKE_CURRENT_LIST_DIR}/../../../target/release")

add_executable(ibus-engine-inputkey inputkey.c ../Settings/inputkey_settings.c)
target_include_directories(ibus-engine-inputkey PRIVATE ${IBUS_INCLUDE_DIRS} ../../../Boundary/NativeABI ../Settings)
target_compile_options(ibus-engine-inputkey PRIVATE ${IBUS_CFLAGS_OTHER})
target_link_directories(ibus-engine-inputkey PRIVATE ${IBUS_LIBRARY_DIRS})
target_link_libraries(ibus-engine-inputkey PRIVATE ${IBUS_LIBRARIES} "${INPUTKEY_NATIVE_LIB}")

install(TARGETS ibus-engine-inputkey DESTINATION libexec)
install(FILES inputkey.xml DESTINATION share/ibus/component)
