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

## Browser Connector Layer (Task 3.1)

### Chrome/Edge CDP Connector (`cdp.rs`)
- Real browser detection via CDP HTTP endpoints (`/json/version`, `/json/list`)
- Connection management with proper state tracking
- Tab retrieval from CDP targets
- Tab operations: close, activate, create via CDP HTTP API
- Page content fetching with basic HTML parsing
- Support for custom debug ports

### Firefox WebExtensions Connector (`firefox.rs`)
- Firefox profile detection across Windows, Linux, and macOS
- Native messaging host registration checking
- Connection state management
- Data structures for Firefox tabs and bookmarks
- Conversion utilities for Firefox data to core types

### Browser Connector Manager (`lib.rs`)
- Unified interface for managing multiple browser connections
- Browser instance lifecycle management with status tracking
- Automatic browser detection on common ports
- Privacy mode filtering integration
- Methods for connecting to all detected browsers
- Aggregated tab/bookmark retrieval across browsers

## Tab Data Extraction and Filtering (Task 3.3)

### Enhanced Privacy Mode Filter (`privacy_filter.rs`)
- Extended filtering beyond just `is_private` flag
- URL pattern detection for privacy-sensitive URLs (settings, passwords, history pages)
- Filtering for browser internal pages (`chrome://`, `edge://`, `about:`)
- Filtering for extension pages (`chrome-extension://`, `moz-extension://`)
- Configurable filter options via `PrivacyFilterConfig`
- `FilterStats` for reporting filtering statistics

### Tab State Monitor (`tab_monitor.rs`)
- `TabMonitor` for tracking tab state changes across browsers
- Detection of tab events: Created, Closed, Navigated, TitleChanged, Activated, LoadingStateChanged
- Event history with configurable retention
- Event subscription via async channels
- Methods to get recently closed tabs and event statistics

### Tab Information Extractor (`tab_extractor.rs`)
- `TabExtractor` for enhanced tab metadata extraction
- URL parsing to extract domain, subdomain, path, and query parameters
- Automatic tab categorization (Search, SocialMedia, Video, News, Shopping, Development, etc.)
- Methods to group tabs by domain or category
- `TabStats` for comprehensive tab statistics

### BrowserConnectorManager Integration
- `with_config()` constructor for custom privacy and monitor configuration
- `get_extended_tabs()` and `get_all_extended_tabs()` for enhanced tab info
- `get_tabs_by_domain()` and `get_all_tabs_by_domain()` for domain grouping
- `get_tabs_by_category()` and `get_all_tabs_by_category()` for category grouping
- `get_tab_stats()` and `get_all_tab_stats()` for statistics
- `update_tab_monitor()` for change detection
- `get_filter_stats()` for privacy filter statistics
- `get_recently_closed_tabs()` and `get_recent_tab_events()` for history access

## Build System

- Cargo workspace for Rust modules
- CMake for C++ AI processor with platform detection
- Compile-time UI framework selection via Cargo features
