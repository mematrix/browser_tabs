# Web Page Manager - Project Structure

## Project Structure (Rust Core + C++ AI + Multi-UI Framework)

- `core/` - Core types, errors, and FFI interfaces
- `browser-connector/` - Browser connection via CDP (Chrome/Edge) and WebExtensions (Firefox)
- `data-access/` - SQLite database layer with FTS5 full-text search
- `ai-processor/` - C++ AI content processing (content analyzer, similarity calculator, group suggester)
- `ai-processor-ffi/` - Rust FFI bindings for C++ AI processor
- `ui-manager/` - Multi-UI framework support (Flutter, WinUI, GTK, Qt) with compile-time selection
- `page-manager/` - Unified page management for tabs and bookmarks (data merging, association matching, synchronization)

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

## Bookmark Import and Management (Task 3.4)

### Bookmark Import Module (`bookmark_import.rs`)

#### Bookmark Import Wizard and Auto-Detection (Requirement 2.1)
- `BookmarkImporter` struct with methods to detect bookmark sources from Chrome, Edge, and Firefox
- Platform-specific path detection for bookmark files (Windows, Linux, macOS)
- `detect_bookmark_sources()` - auto-detects all installed browsers with bookmarks
- `import_from_browser()` - imports bookmarks from a specific browser
- `import_all()` - imports bookmarks from all detected sources

#### Bookmark Data Parsing and Standardization
- Chrome/Edge JSON bookmark parsing with recursive folder traversal
- Firefox SQLite database parsing (places.sqlite)
- Chrome timestamp conversion (Windows epoch to Unix)
- Standardized `BookmarkInfo` output format across all browsers

#### Bookmark Validation and Status Checking (Requirement 2.2)
- `BookmarkValidator` struct for checking bookmark accessibility
- `validate_bookmark()` - validates a single bookmark's URL
- `validate_batch()` - validates multiple bookmarks concurrently
- `ValidationReport` - comprehensive report with accessibility statistics
- Support for detecting redirects, timeouts, 404s, and network errors

#### Integration with BrowserConnectorManager
- `create_bookmark_importer()` - factory method for creating importers
- `create_bookmark_validator()` - factory method for creating validators
- `import_all_bookmarks()` - convenience method for full import
- `validate_bookmarks()` - convenience method for batch validation

## Bookmark Content Analyzer (Task 5.1)

### Bookmark Content Analyzer Module (`bookmark_content_analyzer.rs`)

#### Web Page Content Fetching Functionality
- `fetch_bookmark_content()` - Fetches full page content for a single bookmark
- `fetch_batch()` - Batch processing with concurrent requests
- Configurable timeouts, max content size, and concurrent request limits
- `BookmarkContentAnalyzerConfig` - Configuration options for request timeout, max concurrent requests, max content size, user agent, redirect handling

#### Bookmark Accessibility Verification Mechanism
- `validate_accessibility()` - Lightweight HEAD request to check URL accessibility
- Proper handling of HTTP status codes (200-299, 301/302/307/308, 403, 404)
- Timeout and network error detection
- Redirect URL tracking

#### Page Metadata Extraction Functionality
- `extract_metadata()` - Extracts comprehensive metadata from HTML content
- Supports: title, description, author, published/modified dates, language, og:image, canonical URL, site name
- HTML entity decoding for common entities (&amp;, &lt;, &gt;, &quot;, &nbsp;, etc.)
- Text content extraction (strips scripts and styles)
- Image and link extraction from HTML
- Keyword extraction from meta tags

#### Data Structures
- `BookmarkContentAnalyzer` - Main analyzer struct
- `BookmarkContentAnalyzerConfig` - Configuration options
- `BookmarkContentResult` - Result for single bookmark analysis (status, content, metadata, response time)
- `BatchAnalysisResult` - Result for batch processing (total, successful, failed counts, duration)

#### Integration with BrowserConnectorManager
- `create_bookmark_content_analyzer()` - Factory method for creating analyzers
- `create_bookmark_content_analyzer_with_config()` - Factory method with custom configuration
- `fetch_bookmark_content()` - Convenience method for single bookmark content fetching
- `fetch_bookmark_content_batch()` - Convenience method for batch content fetching
- `validate_bookmark_accessibility()` - Convenience method for accessibility validation

## Batch Bookmark Analysis and Deduplication (Task 5.2)

### Batch Bookmark Processor (`bookmark_content_analyzer.rs`)

#### New Components

**BatchAnalysisConfig** - Configuration for batch bookmark analysis:
- `similarity_threshold` - Threshold for detecting duplicates (default 0.8)
- `detect_exact_duplicates` - Enable exact URL duplicate detection
- `detect_similar_content` - Enable similar content duplicate detection
- `detect_redirect_chains` - Enable redirect chain duplicate detection
- `max_concurrent_fetches` - Maximum concurrent content fetches

**BatchBookmarkProcessor** - Main processor for batch analysis:
- `analyze_batch()` - Analyzes batches of bookmarks concurrently
- `detect_exact_url_duplicates()` - Detects bookmarks with identical normalized URLs
- `detect_redirect_duplicates()` - Detects bookmarks that redirect to the same final URL
- `detect_similar_content_duplicates()` - Detects bookmarks with similar content using text similarity
- `merge_overlapping_groups()` - Merges overlapping duplicate groups
- `generate_merge_suggestions()` - Generates merge suggestions for duplicate groups

**BatchBookmarkAnalysis** - Result structure containing:
- Total and unique bookmark counts
- Duplicate groups with type and similarity scores
- Merge suggestions with confidence scores
- Individual bookmark analysis results
- Timing information (started_at, completed_at, total_duration_ms)

**MergeSuggestion** - Suggestion for merging duplicates:
- `keep_bookmark` - Recommended bookmark to keep
- `remove_bookmarks` - Bookmarks recommended to remove
- `reason` - Explanation for the suggestion
- `confidence` - Confidence score (0.0 - 1.0)
- `merged_metadata` - Combined metadata from all bookmarks

**MergedBookmarkMetadata** - Merged metadata from multiple bookmarks:
- `best_title` - Best title from all bookmarks (longest non-empty)
- `combined_keywords` - Combined keywords from all bookmarks
- `suggested_folder_path` - Best folder path suggestion (deepest)
- `combined_description` - Combined description

#### Key Algorithms

**URL Normalization** (`normalize_url`):
- Removes tracking parameters (utm_*, ref=, source=, fbclid=, gclid=)
- Removes www. prefix
- Removes trailing slashes
- Removes URL fragments
- Case-insensitive comparison

**String Similarity** (`string_similarity`):
- Levenshtein distance-based similarity ratio
- Case-insensitive comparison

**Jaccard Similarity** (`jaccard_similarity`):
- Word-based Jaccard index for text content comparison
- Tokenization with filtering of short words

**Content Similarity** (`calculate_content_similarity`):
- Weighted combination of title similarity (0.3), text similarity (0.4), keywords overlap (0.2), description similarity (0.1)

**Best Bookmark Selection** (`select_best_bookmark`):
- Scores bookmarks based on title length, favicon presence, access history, folder depth, and age
- Selects bookmark with highest score

## AI Content Processor (Task 4.1)

### Basic AI Content Processing Implementation

The AI content processor implementation includes:

- **Text summarization**: Extractive summarization using sentence scoring based on word frequency, with fallback to description or truncated text
- **Keyword extraction**: TF-based analysis with stop word filtering, merging meta keywords with extracted keywords from text and title
- **Content classification**: Classifies pages into 8 categories (Article, Video, Documentation, SocialMedia, Shopping, News, Reference, Other) based on title and content patterns
- **Similarity calculation**: Multiple methods including cosine similarity, Jaccard similarity, n-gram similarity, and combined similarity for robust content comparison

### Content Analyzer (`content_analyzer.cpp`)
- HTML text extraction with script/style removal and entity decoding
- Title and meta description extraction from HTML
- Meta keywords and image/link extraction
- Language detection supporting English, Chinese, Spanish, French, German, Russian, Arabic
- Reading time estimation (200 words/min for word-based, 300 chars/min for character-based languages)
- Content type classification based on URL patterns and content analysis
- Extractive summarization using sentence scoring with word frequency and position weighting
- Keyword extraction using term frequency with stop word filtering
- Key point extraction for generating content highlights

### Similarity Calculator (`similarity_calculator.cpp`)
- Cosine similarity calculation using TF vectors
- Jaccard similarity for keyword set comparison
- N-gram similarity (bigrams, trigrams) for better phrase matching
- Combined similarity using weighted multiple methods
- TF-IDF calculation for document corpus analysis
- Summary similarity combining text, key points, content type, and language
- Document search with similarity threshold filtering

### Group Suggester (`group_suggester.cpp`)
- Content-based grouping using similarity clustering
- Domain-based grouping by extracting domains from URLs
- Topic-based grouping using primary keywords
- Group merging based on overlap threshold
- Automatic group name generation from common words
- Group description generation with keyword analysis

### AI Content Processor Interface (`ai_processor.cpp`)
- Unified processor with three processing modes: Basic, Enhanced, Auto
- Integration of content analyzer, similarity calculator, and group suggester
- Summary generation with confidence scoring
- Keyword extraction merging meta keywords with text analysis
- Content classification with category hierarchy
- Content relevance scoring between pages
- Page structure analysis with metadata extraction
- Topic identification from keywords
- Processing capabilities reporting

## Enhanced AI Processor and Smart Grouping (Task 4.3)

### C++ AI Processor Enhancements (ai-processor/)

#### Page Structure Analysis (`AnalyzePageLayout`)
- Counts headings, paragraphs, lists, tables, forms, and media elements
- Extracts heading texts and sections
- Detects navigation, sidebar, and footer elements
- Calculates content density

#### Entity Extraction (`ExtractEntities`)
- Extracts person names (capitalized word sequences)
- Extracts organizations (Inc, Corp, Ltd, etc.)
- Extracts website domains from URLs
- Returns confidence scores and positions

#### Sentiment Analysis (`AnalyzeSentiment`)
- Lexicon-based sentiment analysis
- Returns sentiment label (positive/negative/neutral) and score (-1.0 to 1.0)

#### Topic Extraction (`ExtractTopics`)
- Keyword clustering for topic identification
- Filters duplicates and similar topics

#### Smart Grouping Enhancements (GroupSuggester)
- `SuggestGroupsCombined`: Combines content, domain, and topic-based grouping
- `GenerateCrossRecommendations`: Generates cross-content recommendations with relevance scores
- `RankSuggestions`: Ranks group suggestions by quality
- `DetectClusters`: Hierarchical clustering using similarity matrix

### Rust FFI Layer (ai-processor-ffi/)
- Added C-compatible structs for new data types (`CPageStructure`, `CEntityInfo`, `CCrossRecommendation`, `CGroupSuggestion`)
- Implemented FFI functions for all new features
- Added proper memory management (free functions)
- Internal helper functions for page structure analysis, entity extraction, sentiment analysis, and grouping

## Build System

- Cargo workspace for Rust modules
- CMake for C++ AI processor with platform detection
- Compile-time UI framework selection via Cargo features


## Page Unified Manager (Task 6.1)

### New `page-manager` Crate

Unified management of tabs and bookmarks with data merging, association matching, and synchronization.

#### Matcher Module (`matcher.rs`)

**Tab-Bookmark Association Matching (Requirements 6.1, 6.2)**
- `TabBookmarkMatcher` - Main matcher for tab-bookmark associations
- URL normalization for consistent matching (removes trailing slashes, normalizes scheme, handles case)
- Exact URL matching with confidence 1.0
- Domain-based matching with confidence 0.5
- `MatcherConfig` - Configurable matching options (similarity threshold, match types, URL normalization)
- `find_matches_for_tab()` - Find all bookmarks matching a tab
- `find_matches_for_bookmark()` - Find all tabs matching a bookmark
- `build_match_map()` - Build complete match map between tabs and bookmarks

**Content Change Detection (Requirement 6.2)**
- `ContentChangeDetector` - Detects changes between tabs and their matching bookmarks
- `ContentChangeDetection` - Result structure with title/favicon change flags
- `detect_changes()` - Compare single tab-bookmark pair
- `detect_all_changes()` - Detect changes for all matched pairs

#### Sync Module (`sync.rs`)

**Data Synchronization (Requirements 6.2, 6.3)**
- `DataSyncManager` - Main synchronization manager
- `SyncAction` - Enum for sync operations (UpdateBookmark, CreateBookmark, UpdateUnifiedPage)
- `SyncResult` - Result of synchronization with performed actions and errors
- `PageUpdates` - Updates to apply to unified pages

**Sync Operations**
- `generate_sync_actions()` - Generate sync actions from detected changes
- `apply_bookmark_update()` - Apply updates to a bookmark
- `create_bookmark_from_tab_with_inheritance()` - Create bookmark with data inheritance
- `merge_to_unified_page()` - Merge tab and bookmark into unified page
- `batch_merge()` - Batch merge tabs and bookmarks into unified pages

**Sync Queue**
- `SyncQueue` - Queue for managing pending sync items
- `PendingSyncItem` - Pending sync item with change detection and suggested action
- Approval workflow for user-controlled synchronization

#### Unified Manager Module (`unified_manager.rs`)

**Main Management Interface (Requirements 6.1, 6.2, 6.3)**
- `PageUnifiedManager` - Main component for unified tab/bookmark management
- `PageUnifiedManagerConfig` - Configuration options
- `TabAssociationStatus` - Association status for a tab (has_bookmark, pending_changes)
- `UnifiedManagerStats` - Statistics about manager state

**Data Management**
- `update_tabs()` - Update with new tab data
- `update_bookmarks()` - Update with new bookmark data
- `update_all()` - Update both tabs and bookmarks
- Automatic association refresh and change detection

**Query Methods**
- `get_unified_pages()` - Get all unified pages
- `get_unified_page_by_id()` / `get_unified_page_by_url()` - Get specific page
- `get_tab_association_status()` - Get association status for a tab
- `get_tabs_with_bookmarks()` - Get tabs that have matching bookmarks
- `get_tabs_with_pending_changes()` - Get tabs with detected changes
- `find_bookmarks_for_tab()` / `find_tabs_for_bookmark()` - Find matches

**Sync Methods**
- `pending_sync_count()` / `get_pending_sync_items()` - Query pending syncs
- `approve_sync_item()` / `approve_all_sync_items()` - Approve syncs
- `execute_approved_syncs()` - Execute approved synchronizations
- `clear_pending_syncs()` - Clear pending items

**Bookmark Creation (Requirement 6.3)**
- `create_bookmark_from_tab()` - Create bookmark with data inheritance from unified page

#### Property Tests (`tests/property_tests.rs`)

**Correctness Properties Validated**
- Property 15: Tab-Bookmark Association Consistency - Tabs and bookmarks with same URL match correctly
- Property 16: Data Inheritance Integrity - Bookmarks inherit content summary and keywords from tabs
- URL normalization idempotence
- Match confidence ordering (exact > domain)
- Batch merge URL preservation
- Content change detection accuracy


## Unified Search Functionality (Task 6.4)

### Search Module (`search.rs`)

#### Cross-Data-Source Unified Search Interface (`UnifiedSearchManager`)
- Searches across active tabs, bookmarks, unified pages, tab history, and archived content
- Uses FTS5 full-text search for database queries
- In-memory search for cached tabs and bookmarks
- `search()` - Main search entry point with options
- `update_tabs()` / `update_bookmarks()` - Update cached data for in-memory search
- Relevance scoring based on title match, URL match, and keyword match
- Deduplication by URL with priority for higher relevance and source type

#### Search Result Sorting and Filtering (`SearchFilter`, `SearchSortOrder`, `SearchOptions`)
- `SearchFilter` - Filter by source type (tabs, bookmarks, history, archives), browser type, date range, keywords
- `SearchSortOrder` - Sort by relevance, recency (newest/oldest first), or title (asc/desc)
- `SearchOptions` - Pagination (limit, offset), sort order, filter, snippet inclusion
- `SearchResultItem` - Unified result item with id, url, title, source type, relevance score, snippet, keywords
- `SearchResultSource` - Enum for result sources (ActiveTab, Bookmark, History, Archive, UnifiedPage)
- `SearchResults` - Results container with total count, items, search time, and grouping by source

#### Search History and Suggestions (`SearchHistoryEntry`, `SearchSuggestion`)
- `record_search()` - Records search queries with timestamps and result counts
- `get_search_history()` - Retrieves recent search history
- `clear_search_history()` - Clears search history
- `get_suggestions()` - Provides suggestions based on search history, page titles, and keywords
- `SuggestionType` - Enum for suggestion sources (History, Title, Keyword, Url)
- Deduplication and ranking of suggestions by score

#### In-Memory Search Methods in `PageUnifiedManager`
- `search_pages()` - Simple text search across cached unified pages (title, URL, keywords, category)
- `search_pages_filtered()` - Search with browser type and source type filters
- `get_cached_tabs()` / `get_cached_bookmarks()` - Access cached data for external search managers

#### Key Features
- Unified search across all data sources (Requirement 6.5)
- FTS5 full-text search integration for database queries
- Configurable filtering by source type, browser, date range, and keywords
- Multiple sort options (relevance, recency, title)
- Pagination support for large result sets
- Search history tracking with deduplication
- Auto-complete suggestions from multiple sources
- Result deduplication with source priority (ActiveTab > Bookmark > UnifiedPage > History > Archive)


## Tab History Manager (Task 7.1)

### History Module (`page-manager/src/history.rs`)

#### Tab Close Event Listening (Requirement 7.1)
- `process_tab_events()` - Processes TabEvent::Closed events from the TabMonitor
- `should_save_tab()` - Filters out private tabs, internal browser pages, and tabs that were open for less than the minimum lifetime
- Configurable minimum tab lifetime before saving to history
- Option to include/exclude internal browser pages (chrome://, edge://, about:, etc.)

#### History Record Saving and Management (Requirement 7.2)
- `save_closed_tab()` - Saves complete tab information including title, URL, close time, favicon, browser type, and content summary
- `register_content_summary()` - Allows enriching history entries with AI-generated content summaries
- In-memory cache with configurable max entries (`max_cache_entries`)
- Session info tracking (session ID, window ID, tab index)
- `TabHistoryManagerConfig` - Configuration options for cache size, auto-save, minimum tab lifetime, internal page handling, retention policy

#### History Query and Filtering (Requirement 7.3)
- `get_history()` - Query with comprehensive filtering (browser type, date range, URL/title patterns, pagination)
- `search()` - Full-text search across title, URL, and content summary with relevance-based sorting
- `get_recent()` - Get most recent history entries
- `get_by_browser()` - Filter by browser type
- `get_in_time_range()` - Filter by date range
- `get_recently_closed()` - Get entries closed within N minutes
- `count()` - Count entries matching a filter
- `get_by_id()` - Get specific entry by ID

#### History Management
- `delete()` - Delete entry by ID
- `delete_older_than()` - Delete entries older than a timestamp
- `apply_retention_policy()` - Automatic cleanup based on age and entry count with importance-based preservation
- `clear()` - Clear all history entries
- Importance calculation based on content summary presence, keywords, and recency

#### Statistics and Analytics
- `get_stats()` - Comprehensive statistics including entry counts by browser, session saves/restores, date ranges
- `get_entries_by_domain()` - Group entries by domain
- `get_top_domains()` - Get most frequently closed domains
- `HistoryManagerStats` - Statistics structure with cached entries, entries by browser, session saves/restores, oldest/newest entry timestamps

#### Data Structures
- `TabHistoryManager` - Main manager struct with in-memory cache and content summary registry
- `TabHistoryManagerConfig` - Configuration options
- `HistoryManagerStats` - Statistics about the history manager
- Integration with `TabMonitor` for event subscription
- Uses core types: `HistoryEntry`, `HistoryFilter`, `RetentionPolicy`, `SessionInfo`


## Tab Restoration and Cleanup (Task 7.3)

### History Module Enhancements (`page-manager/src/history.rs`)

#### Tab Restoration (Requirement 7.4)
- `restore_tab()` - Restores a history tab in a specified browser using the BrowserConnector trait
- `restore_tabs_batch()` - Batch restoration of multiple tabs with individual result tracking
- `get_restore_url()` - Get URL for manual restoration when automatic fails
- `get_restore_urls()` - Get URLs for multiple history entries
- `RestoreResult` - Result structure tracking restoration outcomes (history_id, new_tab_id, target_browser, success, error, restored_at)

#### Automatic Cleanup (Requirement 7.5)
- `run_auto_cleanup()` - Runs cleanup using the default retention policy
- `cleanup_with_policy()` - Runs cleanup with a custom retention policy
- `needs_cleanup()` - Checks if cleanup is needed based on current state
- `preview_cleanup()` / `preview_cleanup_with_policy()` - Preview what would be deleted without actually deleting
- `CleanupResult` - Detailed statistics (deleted_by_age, deleted_by_limit, preserved_important, remaining_entries, cleaned_at)
- Enhanced `TabHistoryManagerConfig` with auto-cleanup settings (auto_cleanup_on_startup, auto_cleanup_interval_hours)
- Importance-based preservation during cleanup (entries with content summaries and keywords are prioritized)

#### Export and Backup
- `export()` - Export history in JSON, CSV, or HTML formats
- `export_filtered()` - Export filtered history entries
- `import()` - Import history from JSON data
- `save_to_file()` - Save history to a file
- `load_from_file()` - Load history from a file
- `ExportFormat` - Enum for export formats (Json, Csv, Html)
- `ExportedHistory` - Exported data structure with metadata and entries
- `ExportMetadata` - Export metadata (exported_at, app_version, entry_count, date_range, format)

#### Data Structures
- `RestoreResult` - Result of tab restoration operation
- `CleanupResult` - Statistics about cleanup operation
- `ExportFormat` - Export format enum
- `ExportedHistory` - Exported history data structure
- `ExportMetadata` - Metadata for exported history

#### Enhanced Statistics
- `session_cleanups` - Number of entries cleaned up in current session
- `last_cleanup` - Timestamp of last cleanup operation


## Remote Tab Controller (Task 8.1)

### Remote Controller Module (`page-manager/src/remote_controller.rs`)

#### Core Tab Operations (Requirement 1.5)
- `close_tab()` - Close a tab in a browser with optional tab info for undo support
- `activate_tab()` - Activate/focus a tab in a browser
- `create_tab()` - Create a new tab with a URL
- All operations work with both direct `BrowserConnector` trait and `BrowserConnectorManager`
- `close_tab_via_manager()` / `activate_tab_via_manager()` / `create_tab_via_manager()` - Operations using BrowserConnectorManager

#### Operation Result Verification and Error Handling
- `TabOperationResult` - Result structure with operation record, new tab ID, and verification status
- `OperationStatus` - Enum for operation status (Success, Failed, PendingVerification, RolledBack)
- `TabOperationType` - Enum for operation types (Close, Activate, Create)
- Proper error propagation using existing error types
- Logging of all operations with tracing

#### Operation History and Undo Mechanism
- `TabOperationRecord` - Record for tracking all operations (id, type, browser, tab_id, url, title, status, timestamp, undoable, related_operation_id)
- Configurable history size via `RemoteTabControllerConfig` (default 100 operations)
- `undo_close()` - Reopen a closed tab by creating a new tab with the same URL
- `undo_create()` - Close a created tab
- `undo_last()` - Undo the most recent undoable operation
- Operations linked via `related_operation_id` for tracking undo relationships
- `get_history()` - Get full operation history
- `get_recent_operations()` - Get recent operations
- `get_undoable_operations()` - Get operations that can be undone

#### Statistics and Management
- `RemoteControllerStats` - Statistics structure (total_operations, successful_operations, failed_operations, operations_by_type, operations_by_browser, undo_operations, history_size)
- `get_stats()` - Get controller statistics
- `clear_history()` - Clear operation history
- `get_operation()` - Get operation by ID
- `get_operations_for_browser()` - Get operations for a specific browser
- `get_operations_by_type()` - Get operations of a specific type
- `get_failed_operations()` - Get failed operations

#### Configuration
- `RemoteTabControllerConfig` - Configuration options:
  - `max_history_size` - Maximum operations in history (default 100)
  - `verify_operations` - Whether to verify operations after execution
  - `verification_timeout_ms` - Timeout for verification (default 5000ms)
  - `enable_undo` - Whether to enable undo functionality
  - `max_retry_attempts` - Maximum retry attempts for failed operations (default 2)

#### Design Properties
- Property 4: Remote control operation atomicity - Operations either fully succeed or fully fail, maintaining original state


## Cross-Browser Tab Migration (Task 8.3)

### Remote Controller Module Enhancements (`page-manager/src/remote_controller.rs`)

#### New Types for Cross-Browser Migration

**MigrationType** - Enum for migration operation types:
- `Full` - Full migration with session state preservation attempt
- `UrlOnly` - URL-only migration (fallback when session state cannot be preserved)
- `Export` - Export URLs for manual import (fallback when direct migration fails)

**MigrationStatus** - Status tracking for migrations:
- `Success` - Migration completed successfully
- `SuccessWithFallback { fallback_type }` - Migration completed with fallback method
- `Failed(String)` - Migration failed with error message
- `Pending` - Migration is pending
- `RolledBack` - Migration was rolled back

**SessionState** - Captures session state for migration (Requirement 8.3):
- `url` - URL of the tab
- `title` - Title of the tab
- `scroll_position` - Scroll position (if available)
- `form_data` - Form data (if available and safe to transfer)
- `cookies` - Cookies associated with the page (domain-specific)
- `local_storage` / `session_storage` - Storage data (if available)
- `captured_at` - Timestamp when state was captured
- `has_preserved_data()` - Check if session state has preserved data beyond URL/title

**CookieInfo** - Cookie information structure for session preservation

**MigrationRecord** - Complete record of a migration operation:
- Source/target browser, tab IDs, URL, title
- Migration type and status
- Session state and preservation status
- Timestamps (initiated_at, completed_at)
- Rollback capability flag
- Error message if failed

**MigrationResult** - Result wrapper with:
- Migration record
- Fallback usage flag
- Fallback data (if export fallback was used)

**FallbackData** - Export data for when direct migration fails (Requirement 8.4):
- `fallback_type` - Type of fallback (UrlExport, HtmlBookmarkExport, JsonExport, ClipboardCopy)
- `urls` - List of URL export entries
- `format` - Export format (PlainText, Html, Json)
- `export_content` - Generated export content
- `instructions` - User instructions for manual import

**MigrationConfig** - Configuration options for migration:
- `preserve_session_state` - Whether to attempt session state preservation (default true)
- `close_source_tab` - Whether to close source tab after successful migration (default true)
- `activate_target_tab` - Whether to activate new tab in target browser (default true)
- `timeout_ms` - Timeout for migration operations (default 10000ms)
- `auto_fallback` - Whether to automatically use fallback on failure (default true)
- `preferred_fallback` - Preferred fallback type (default UrlExport)

#### New Methods for Cross-Browser Migration

**migrate_tab()** - Main migration method (Requirements 8.2, 8.3, 8.4):
1. Captures session state from source tab
2. Creates tab in target browser with the same URL
3. Optionally closes source tab
4. Provides rollback capability
5. Falls back to alternative methods on failure

**migrate_tabs_batch()** - Batch migration for multiple tabs

**generate_fallback_export()** - Creates export data in PlainText, JSON, or HTML bookmark format (Requirement 8.4)

**rollback_migration()** - Rolls back a migration by closing target tab and reopening in source browser (Requirement 8.5)

**verify_migration()** - Verifies a migration was successful by checking if target tab exists (Requirement 8.5)

**get_migration_history()** / **get_recent_migrations()** / **get_rollbackable_migrations()** - Migration history management

**clear_migration_history()** - Clear migration history

#### Enhanced Statistics
- `cross_browser_migrations` - Number of cross-browser migrations performed
- `fallback_operations` - Number of fallback operations used

#### Requirements Covered
- **8.2**: Cross-browser tab migration with safe movement
- **8.3**: Session state and login information preservation (best-effort)
- **8.4**: Fallback solutions (URL export/import) when API limitations prevent direct migration
- **8.5**: Operation verification and rollback capability
