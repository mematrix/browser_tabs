cmake_minimum_required(VERSION 3.20)
project(WebPageManager)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Build configuration
option(BUILD_TESTS "Build tests" ON)
option(BUILD_AI_PROCESSOR "Build AI processor" ON)

# Platform detection
if(WIN32)
    set(PLATFORM_WINDOWS TRUE)
elseif(UNIX AND NOT APPLE)
    set(PLATFORM_LINUX TRUE)
elseif(APPLE)
    set(PLATFORM_MACOS TRUE)
endif()

# UI Framework selection (compile-time choice)
option(USE_FLUTTER_UI "Use Flutter UI" OFF)
option(USE_WINUI_UI "Use WinUI 3 (Windows only)" OFF)
option(USE_GTK_UI "Use GTK UI (Linux)" OFF)
option(USE_QT_UI "Use Qt UI (Cross-platform)" OFF)

# Ensure only one UI framework is selected
set(UI_FRAMEWORK_COUNT 0)
if(USE_FLUTTER_UI)
    math(EXPR UI_FRAMEWORK_COUNT "${UI_FRAMEWORK_COUNT} + 1")
endif()
if(USE_WINUI_UI)
    math(EXPR UI_FRAMEWORK_COUNT "${UI_FRAMEWORK_COUNT} + 1")
endif()
if(USE_GTK_UI)
    math(EXPR UI_FRAMEWORK_COUNT "${UI_FRAMEWORK_COUNT} + 1")
endif()
if(USE_QT_UI)
    math(EXPR UI_FRAMEWORK_COUNT "${UI_FRAMEWORK_COUNT} + 1")
endif()

if(UI_FRAMEWORK_COUNT GREATER 1)
    message(FATAL_ERROR "Only one UI framework can be selected at compile time")
elseif(UI_FRAMEWORK_COUNT EQUAL 0)
    message(STATUS "No UI framework selected, defaulting to Flutter")
    set(USE_FLUTTER_UI ON)
endif()

# Add subdirectories
if(BUILD_AI_PROCESSOR)
    add_subdirectory(ai-processor)
endif()

if(USE_WINUI_UI AND PLATFORM_WINDOWS)
    add_subdirectory(ui/winui)
endif()

if(USE_GTK_UI AND PLATFORM_LINUX)
    add_subdirectory(ui/gtk)
endif()

if(USE_QT_UI)
    add_subdirectory(ui/qt)
endif()

# Flutter UI is handled separately via its own build system
if(USE_FLUTTER_UI)
    message(STATUS "Flutter UI will be built using Flutter's build system")
endif()