cmake_minimum_required(VERSION 3.16)
project(inputkey-fcitx5 VERSION 5.2.0 LANGUAGES C CXX)

include(GNUInstallDirs)
find_package(Fcitx5Core REQUIRED)

set(INPUTKEY_NATIVE_LIB_DIR "" CACHE PATH "Directory containing the Rust InputKey native library")
find_library(INPUTKEY_NATIVE_LIB NAMES inputkey_cabi REQUIRED
    HINTS "${INPUTKEY_NATIVE_LIB_DIR}" "${CMAKE_CURRENT_LIST_DIR}/../../../target/release")

add_library(inputkey MODULE inputkey.cpp ../Settings/inputkey_settings.c)
target_compile_features(inputkey PRIVATE cxx_std_17)
target_include_directories(inputkey PRIVATE ../../../Boundary/NativeABI ../Settings)
target_link_libraries(inputkey PRIVATE Fcitx5::Core "${INPUTKEY_NATIVE_LIB}")
set_target_properties(inputkey PROPERTIES PREFIX "" OUTPUT_NAME "inputkey")

install(TARGETS inputkey DESTINATION "${CMAKE_INSTALL_LIBDIR}/fcitx5")
install(FILES inputkey.conf DESTINATION share/fcitx5/addon)
install(FILES inputkey-im.conf DESTINATION share/fcitx5/inputmethod)
