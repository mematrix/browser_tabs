# Browser Tabs Manager Project

## Overview
Browser tabs management application with Rust core and multiple UI implementations.

## Architecture
- Core logic in Rust
- UI implementations: WinUI 3, Flutter
- FFI bindings for C++ integration (providing the AI functionality)

## Tech Stack
- Rust (core library)
- WinUI 3 (Windows native UI)
- Flutter (cross-platform UI)
- C++ (AI integration)

## Conventions
- Use snake_case for Rust functions
- Keep FFI layer minimal and safe
- Use English for all documentation and comments

## Building
- Rust(Build All): `cargo build`
- Rust(Build special package): `cargo build --package <package_name>`
- Rust(Run All Tests): `cargo test`
- C++(Config, usually run once): `cmake -S ai-processor -B build`
- C++(Build): `cmake --build build`

## Notes
The original project was created by Kiro, its design docs are available at ".kiro/specs/web-page-manager/*.md".

