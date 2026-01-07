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

## Data Access Layer (Task 2.3)

### Database Schema and Migrations
- Enhanced `schema.rs` with version tracking via `schema_migrations` table
- Added migration infrastructure with `Migration` struct and `MIGRATIONS` array
- Extended schema with additional tables: `tab_history`, `content_archives`
- Added FTS5 indexes for pages, archives, and history with porter tokenizer

### Repository Implementations
- `PageRepository` - CRUD operations for unified pages with pagination and access tracking
- `GroupRepository` - Smart group management with page-group relations
- `HistoryRepository` - Tab history with filtering and cleanup
- `ArchiveRepository` - Content archive storage with size tracking
- `UnifiedSearchRepository` - Cross-data-source unified search

### FTS5 Full-Text Search
- Three FTS5 virtual tables: `pages_fts`, `archives_fts`, `history_fts`
- Automatic index maintenance via triggers (insert/update/delete)
- Porter stemming and unicode61 tokenization for better search results
- Search methods on all repositories with ranking

### Data Caching Strategy
- `LruCache<K, V>` - Generic LRU cache with TTL support
- `DataCache` - Thread-safe cache manager for pages, summaries, and groups
- `CachedPageRepository` - Cache-aware page repository wrapper
- Configurable cache sizes and TTLs via `CacheConfig`

### DatabaseManager Enhancements
- Automatic migration execution on startup
- Factory methods for all repositories
- Database statistics and optimization (VACUUM/ANALYZE)
- Cache management integration

## Build System

- Cargo workspace for Rust modules
- CMake for C++ AI processor with platform detection
- Compile-time UI framework selection via Cargo features
