# Generated Cmake Pico project file

cmake_minimum_required(VERSION 3.13)

set(CMAKE_C_STANDARD 11)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# Initialise pico_sdk from installed location
# (note this can come from environment, CMake cache etc)

# == DO NEVER EDIT THE NEXT LINES for Raspberry Pi Pico VS Code Extension to work ==
if(WIN32)
    set(USERHOME $ENV{USERPROFILE})
else()
    set(USERHOME $ENV{HOME})
endif()
set(sdkVersion 2.0.0)
set(toolchainVersion 13_2_Rel1)
set(picotoolVersion 2.0.0)
set(picoVscode ${USERHOME}/.pico-sdk/cmake/pico-vscode.cmake)
if (EXISTS ${picoVscode})
    include(${picoVscode})
endif()
# ====================================================================================
set(PICO_BOARD pico_w CACHE STRING "Board type")

# Pull in Raspberry Pi Pico SDK (must be before project)
include(pico_sdk_import.cmake)

project(PicoLightShow C CXX ASM)

# Initialise the Raspberry Pi Pico SDK
pico_sdk_init()

#add_compile_options(-Wall -Wextra -Wpedantic)

# Add executable. Default name is the project name, version 0.1

add_executable(PicoLightShow 
    main.cpp
    src/light_show_runner/light_show_runner.cpp
    src/light_effects/light_effect_base.cpp
    src/light_effects/running_point.cpp
    src/light_effects/custom_pattern.cpp
    src/persistent_settings/persistent_settings.cpp
    src/www_server/ssi.cpp
    src/www_server/cgi.cpp
    src/www_server/www_server.cpp
    dhcpserver/dhcpserver.c
    dnsserver/dnsserver.c)

file(MAKE_DIRECTORY ${CMAKE_CURRENT_LIST_DIR}/generated)

pico_generate_pio_header(PicoLightShow ${CMAKE_CURRENT_LIST_DIR}/ws2812.pio OUTPUT_DIR ${CMAKE_CURRENT_LIST_DIR}/generated)

pico_set_program_name(PicoLightShow "PicoLightShow")
pico_set_program_version(PicoLightShow "0.1")

# Modify the below lines to enable/disable output over UART/USB
pico_enable_stdio_uart(PicoLightShow 0)
pico_enable_stdio_usb(PicoLightShow 1)

# Add the standard library to the build
target_link_libraries(PicoLightShow
        pico_stdlib)

# Add the standard include files to the build
target_include_directories(PicoLightShow PRIVATE
  ${CMAKE_CURRENT_LIST_DIR}
  ${CMAKE_CURRENT_LIST_DIR}/.. # for our common lwipopts or any other standard includes, if required
)

# Add any user requested libraries
target_link_libraries(PicoLightShow 
        pico_cyw43_arch_lwip_poll
        pico_lwip_http
        pico_multicore
        hardware_pio
        hardware_flash)

pico_add_extra_outputs(PicoLightShow)

