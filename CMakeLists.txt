cmake_minimum_required(VERSION 3.25)

# use MSVC_RUNTIME_LIBRARY
cmake_policy(SET CMP0091 NEW)

# use MSVC_DEBUG_INFORMATION_FORMAT
cmake_policy(SET CMP0141 NEW)

# enable vcpkg
set(CMAKE_TOOLCHAIN_FILE "${CMAKE_CURRENT_SOURCE_DIR}/vcpkg/scripts/buildsystems/vcpkg.cmake" CACHE STRING "Vcpkg toolchain file")

project(MuteApp)

enable_language(CXX)

add_executable(MuteApp WIN32
  src/Audio.cpp
  src/ConfigFile.cpp
  src/HotKey.cpp
  src/Main.cpp
  src/NotifyIcon.cpp
  src/MuteApp.rc
)

set_property(TARGET MuteApp PROPERTY CXX_STANDARD 23)

target_compile_options(MuteApp PRIVATE "$<$<C_COMPILER_ID:MSVC>:/utf-8>")
target_compile_options(MuteApp PRIVATE "$<$<CXX_COMPILER_ID:MSVC>:/utf-8>")

target_compile_definitions(MuteApp PRIVATE "$<$<CXX_COMPILER_ID:MSVC>:UNICODE>")
target_compile_definitions(MuteApp PRIVATE "$<$<CXX_COMPILER_ID:MSVC>:_UNICODE>")

set_property(TARGET MuteApp PROPERTY MSVC_RUNTIME_LIBRARY "MultiThreaded$<$<CONFIG:Debug>:Debug>")
set_property(TARGET MuteApp PROPERTY MSVC_DEBUG_INFORMATION_FORMAT "$<$<CONFIG:Debug>:EditAndContinue>$<$<CONFIG:RelWithDebInfo>:ProgramDatabase>")

target_precompile_headers(MuteApp PRIVATE "$<$<COMPILE_LANGUAGE:CXX>:${CMAKE_CURRENT_SOURCE_DIR}/src/pch.h>")

find_package(cppwinrt CONFIG REQUIRED)
target_link_libraries(MuteApp PRIVATE Microsoft::CppWinRT)
