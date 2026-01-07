cmake_minimum_required(VERSION 3.20)
project(AIProcessor)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Find required packages
find_package(PkgConfig REQUIRED)

# AI Processor library
add_library(ai_processor STATIC
    src/ai_processor.cpp
    src/content_analyzer.cpp
    src/similarity_calculator.cpp
    src/group_suggester.cpp
)

target_include_directories(ai_processor PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}/include
)

# Link with Rust FFI library
target_link_libraries(ai_processor
    ${CMAKE_CURRENT_SOURCE_DIR}/../target/release/libai_processor_ffi.a
)

# Platform-specific libraries
if(WIN32)
    target_link_libraries(ai_processor
        ws2_32
        userenv
        bcrypt
    )
elseif(UNIX)
    target_link_libraries(ai_processor
        pthread
        dl
        m
    )
endif()

# Tests
if(BUILD_TESTS)
    find_package(GTest REQUIRED)
    
    add_executable(ai_processor_tests
        tests/test_ai_processor.cpp
        tests/test_content_analyzer.cpp
    )
    
    target_link_libraries(ai_processor_tests
        ai_processor
        GTest::gtest_main
    )
    
    enable_testing()
    add_test(NAME AIProcessorTests COMMAND ai_processor_tests)
endif()