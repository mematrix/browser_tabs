# Web Page Manager - Project Structure

## Project Structure (Rust Core + C++ AI + Multi-UI Framework)

- `core/` - Core types, errors, and FFI interfaces
- `browser-connector/` - Browser connection via CDP (Chrome/Edge) and WebExtensions (Firefox)
- `data-access/` - SQLite database layer with FTS5 full-text search
- `ai-processor/` - C++ AI content processing (content analyzer, similarity calculator, group suggester)
- `ai-processor-ffi/` - Rust FFI bindings for C++ AI processor
- `ui-manager/` - Multi-UI framework support (Flutter, WinUI, GTK, Qt) with compile-time selection

## Key Components Implemented

- Core data types: `UnifiedPageInfo`, `ContentSummary`, `SmartGroup`, `TabInfo`, `BookmarkInfo`, etc.
- Error handling: `BrowserConnectionError`, `AIProcessingError`, `DataConsistencyError`, `UIError`
- FFI interfaces for cross-language communication
- Browser connector traits and implementations for Chrome, Edge, Firefox
- Privacy mode filtering for excluding incognito tabs
- UI Manager trait with platform-specific implementations
- Database schema with FTS5 full-text search
- Repository pattern for data access

## Build System

- Cargo workspace for Rust modules
- CMake for C++ AI processor with platform detection
- Compile-time UI framework selection via Cargo features
