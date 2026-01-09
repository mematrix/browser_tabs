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


## Content Archiver (Task 9.1)

### Content Archiver Module (`page-manager/src/content_archiver.rs`)

#### HTML Content Extraction and Cleaning (Requirement 3.1)
- `extract_content()` - Main method to extract all content from HTML
- `clean_html()` - Removes scripts, styles, noscript tags, and HTML comments
- `remove_tag_with_content()` - Removes specific HTML tags and their content
- `remove_html_comments()` - Removes HTML comments
- `remove_event_handlers()` - Removes inline event handlers (onclick, onload, etc.)
- `normalize_whitespace()` - Normalizes whitespace in HTML
- `extract_text()` - Extracts plain text from HTML with entity decoding
- `extract_title()` - Extracts page title from `<title>` or `<h1>` tags
- `extract_image_urls()` - Finds all image URLs including srcset and background images
- `extract_other_media_urls()` - Extracts video and audio URLs
- `extract_links()` - Separates internal and external links
- `extract_attribute()` - Extracts attribute values from HTML tags

#### Media File Download and Local Storage (Requirement 3.3)
- `download_media_files()` - Downloads media files with size limits and concurrent download management
- `download_single_media()` - Downloads individual files with checksum generation
- `generate_media_filename()` - Creates unique filenames to avoid collisions
- `guess_mime_type()` - Detects MIME type from URL extension
- Configurable storage path, file size limits, and concurrent download limits via `ContentArchiverConfig`
- `MediaFileInfo` - Information about downloaded media (original URL, local path, file size, MIME type, checksum)
- `MediaDownloadError` - Error information for failed downloads

#### Archive Format and Storage
- `archive_page()` - Main archiving method that extracts content, downloads media, and stores the archive
- `ContentArchive` integration with `ArchiveRepository` for persistence
- Checksum generation using FNV-1a hash for content integrity
- Compression support (placeholder for future implementation with flate2)
- Full-text searchable content storage via FTS5

#### URL Resolution and Processing
- `resolve_url()` - Resolves relative URLs to absolute URLs
- `parse_url()` - Parses URL into (protocol, host, path) components
- `normalize_path()` - Normalizes URL paths (resolves . and ..)
- `extract_domain()` - Extracts domain from URL
- `is_supported_image()` - Checks if URL points to a supported image format

#### Content Analysis
- `count_words()` - Counts words in text
- `estimate_reading_time()` - Estimates reading time based on word count (225 words/minute)
- `calculate_checksum()` - Calculates FNV-1a checksum for content

#### Archive Management
- `get_archive()` - Get archive by ID
- `get_archive_by_page()` - Get archive by page ID
- `search_archives()` - Search archives using full-text search
- `delete_archive()` - Delete archive and associated media files
- `get_total_archive_size()` - Get total size of all archives

#### Configuration (`ContentArchiverConfig`)
- `media_storage_path` - Base directory for storing media files
- `max_media_file_size` - Maximum size for a single media file (default 10MB)
- `max_total_media_size` - Maximum total size for all media files per archive (default 100MB)
- `download_timeout_secs` - Timeout for downloading media files (default 30s)
- `enable_compression` - Whether to compress archived content
- `supported_media_extensions` - Supported media file extensions
- `max_concurrent_downloads` - Maximum concurrent downloads (default 5)

#### Data Structures
- `ContentArchiver` - Main archiver struct
- `ContentArchiverConfig` - Configuration options
- `ArchiveResult` - Result of archiving (archive, downloaded media, failed media, total size, duration)
- `ExtractedContent` - Extracted content (HTML, text, title, image URLs, media URLs, links, reading time, word count)
- `MediaFileInfo` - Downloaded media file information
- `MediaDownloadError` - Failed download information

#### Unit Tests (22 tests)
- Title extraction from `<title>` and `<h1>` tags
- Text extraction with HTML entity decoding
- HTML cleaning (script/style removal)
- Image URL extraction
- URL resolution (absolute, relative, protocol-relative)
- Link extraction (internal/external)
- Word count and reading time estimation
- Attribute extraction
- MIME type detection
- Checksum calculation
- Path normalization
- Domain extraction


## Page Change Detection and Update (Task 9.3)

### Change Detector Module (`page-manager/src/change_detector.rs`)

#### Page Content Change Monitoring System (Requirement 3.5)
- `ChangeDetector` - Main struct for detecting changes in archived web pages
- `check_for_changes()` - Compares archived content with new content and returns detection result
- `calculate_similarity()` - Jaccard similarity algorithm for text comparison (word-based)
- `classify_change()` - Classifies changes based on similarity threshold and content analysis
- `should_check()` - Checks if a URL should be checked based on minimum interval
- `check_batch()` - Batch checking for multiple archives with content fetcher callback

#### Change Classification (`ChangeType`)
- `NoChange` - No significant change detected
- `Minor` - Minor changes (typos, formatting) - similarity >= 0.7
- `Moderate` - Moderate changes (some content updated) - similarity >= 0.4
- `Major` - Major changes (significant content rewrite) - similarity < 0.4
- `StructuralChange` - Page structure changed significantly (title changed)
- `PageUnavailable` - Page no longer accessible
- `Redirected { new_url }` - Page redirects to different URL

#### Incremental Update and Version Management
- `ArchiveVersion` - Version information (version number, archive ID, created_at, checksum, size, change type)
- `add_version()` - Records new version with checksum and change type
- `get_version_history()` - Retrieves version history for a page
- `get_latest_version()` - Gets the latest version for a page
- `compare_versions()` - Line-by-line diff comparison between two versions
- `VersionComparison` - Comparison result (added lines, removed lines, unchanged lines, change percentage)
- Configurable max versions per page with automatic cleanup

#### Update Notification and User Choice Mechanism
- `ChangeNotification` - Notification about detected changes (id, archive_id, url, title, change_type, summary, created_at, is_read, user_action)
- `UpdateOption` - User choices (UpdateArchive, KeepCurrent, CreateNewVersion, DeleteArchive)
- `create_notification()` - Creates notification when changes are detected
- `get_notifications()` / `get_unread_notifications()` - Query notifications
- `mark_notification_read()` - Mark notification as read
- `apply_user_action()` - Apply user's chosen action for a notification
- `clear_notifications()` / `clear_read_notifications()` - Clear notifications

#### Configuration (`ChangeDetectorConfig`)
- `min_check_interval_hours` - Minimum time between checks for the same URL (default 24)
- `change_threshold` - Similarity threshold below which content is considered changed (default 0.85)
- `max_versions_per_page` - Maximum number of versions to keep per page (default 5)
- `auto_check_enabled` - Whether to automatically check for changes (default true)
- `batch_size` - Batch size for checking multiple pages (default 10)

#### Data Structures
- `ChangeDetector` - Main detector struct with config, repository, check times, notifications, and version history
- `ChangeDetectorConfig` - Configuration options
- `ChangeDetectionResult` - Result of change detection (archive_id, url, change_type, similarity_score, change_summary, new_content, checked_at, check_duration_ms)
- `PageChangeContent` - Content from a changed page (html, text, title, checksum, fetched_at)
- `ChangeNotification` - Notification about detected changes
- `ArchiveVersion` - Version information for an archived page
- `VersionComparison` - Comparison result between two versions
- `ChangeDetectorStats` - Statistics (total_notifications, unread_notifications, urls_tracked, total_versions, pages_with_versions)

#### Statistics and Management
- `get_stats()` - Get statistics about change detection activity
- `record_check()` - Record that a URL was checked
- `calculate_checksum()` - Calculate FNV-1a checksum for content
- `generate_change_summary()` - Generate human-readable summary of changes

#### Unit Tests (15 tests)
- Similarity calculation (identical, completely different, partial overlap, empty strings)
- Change classification (no change, minor, moderate, major)
- Version comparison
- Checksum calculation
- Change summary generation
- Should check interval logic
- Notification management
- Version history management
- Statistics tracking


## Flutter UI Implementation (Task 10.1)

### Flutter Project Structure (`flutter_ui/`)

A complete cross-platform Flutter UI implementation for the Web Page Manager application.

#### Main Application Structure
- `main.dart` - Application entry point with window manager initialization and provider setup
- `app.dart` - Main app widget with Material 3 theming and go_router navigation
- `theme/app_theme.dart` - Light and dark theme definitions with Material 3 design

#### Data Models (`lib/models/`)
- `page_info.dart` - `UnifiedPageInfo`, `ContentSummary`, `BrowserType`, `PageSourceType` models
- `smart_group.dart` - `SmartGroup` and `GroupType` models for AI-powered grouping
- `search_result.dart` - `SearchResultItem`, `SearchResults`, `SearchSuggestion` models

#### State Management (`lib/providers/`)
- `PageProvider` - Manages page data (tabs, bookmarks, groups) with refresh and CRUD operations
- `SearchProvider` - Manages search state, filters, sort order, history, and suggestions
- `SettingsProvider` - Manages application settings with SharedPreferences persistence

#### Screens (`lib/screens/`)
- `HomeScreen` - Overview with stats cards, quick actions, recent tabs, and smart groups
- `TabsScreen` - Tab management with list/grid views, browser filtering, and batch operations
- `BookmarksScreen` - Bookmark management with category filtering and sorting options
- `SearchScreen` - Unified search with source filters, sort options, and search history
- `HistoryScreen` - Tab history with date grouping, filtering, and restore functionality
- `SettingsScreen` - Application settings for theme, behavior, data, browser, and hotkeys

#### Widgets (`lib/widgets/`)
- `StatsCard` - Statistics display card with icon, label, and value
- `PageListTile` - Unified list tile for pages with favicon, browser indicator, and actions
- `GroupCard` - Smart group display card with type icon and page count
- `BrowserFilterChips` - Browser type filter chips (Chrome, Firefox, Edge, Safari)
- `SearchResultTile` - Search result display with source badge and relevance score

#### Services (`lib/services/`)
- `RustBridge` - FFI bridge to Rust core library (with mock data for development)
- `SystemTrayService` - System tray integration using system_tray plugin
- `NotificationService` - Native notifications using local_notifier plugin
- `HotkeyService` - Global hotkey registration using hotkey_manager plugin

### Rust UI Manager Enhancements (`ui-manager/src/flutter.rs`)

#### FlutterUIConfig
- `enable_system_tray` - Enable system tray integration
- `enable_hotkeys` - Enable global hotkeys
- `enable_notifications` - Enable native notifications
- `window_title` - Window title
- `window_width/height` - Initial window dimensions
- `min_window_width/height` - Minimum window dimensions

#### FlutterUIManager
- `with_config()` - Create manager with custom configuration
- `is_window_visible()` - Check if window is visible
- `is_minimized_to_tray()` - Check if minimized to tray
- `get_current_data()` - Get current UI data
- `get_registered_hotkeys()` - Get registered hotkeys

#### Method Channels
- `web_page_manager/main` - Window management and lifecycle
- `web_page_manager/data` - Page/group data updates
- `web_page_manager/notification` - System notifications
- `web_page_manager/hotkey` - Global hotkey registration
- `web_page_manager/tray` - System tray management

#### Unit Tests (5 tests)
- Manager creation and initialization
- Configuration with custom options
- Capability reporting
- State management (window visibility, tray minimization)

### Dependencies
- `provider` - State management
- `go_router` - Navigation and routing
- `window_manager` - Desktop window management
- `system_tray` - System tray support
- `local_notifier` - Native notifications
- `hotkey_manager` - Global hotkey registration
- `cached_network_image` - Image caching
- `shared_preferences` - Local storage
- `url_launcher` - URL handling
- `intl` - Internationalization
- `ffi` - Rust FFI integration

### Requirements Implemented
- **Requirement 4.1**: Flutter cross-platform UI with consistent experience
- **Requirement 4.2**: System tray with quick access functionality (Windows, Linux, macOS)
- **Requirement 4.3**: Native notifications for tab activity
- **Requirement 6.5**: Unified search across tabs and bookmarks with filtering and sorting


## Unified UI Manager Interface (Task 10.3)

### Enhanced UI Manager Trait (`ui-manager/src/traits.rs`)

#### New Interface Methods
- `hide_main_window()` - Hide the main window without minimizing to tray
- `restore_from_tray()` - Restore application from system tray
- `unregister_global_hotkeys()` - Remove all registered global hotkeys
- `set_theme()` / `get_theme()` - Theme management (Light, Dark, System)
- `set_event_handler()` - Register event handler for UI events
- `get_state()` - Get current UI state

#### New Data Structures
- `NotificationConfig` - Rich notification configuration with title, message, urgency, actions, and timeout
- `NotificationUrgency` - Urgency levels (Low, Normal, High, Critical)
- `NotificationAction` - Action button for notifications
- `UITheme` - Theme options (Light, Dark, System)
- `UIState` - Current UI state (initialized, window_visible, minimized_to_tray, current_theme, registered_hotkey_count, has_event_handler)
- `NoOpEventHandler` - No-op event handler for testing

#### Enhanced UICapabilities
- `supports_custom_decorations` - Custom window decorations support
- `supports_drag_drop` - Drag and drop support

#### UIManagerAdapter
- Generic adapter wrapper for any UIManager implementation
- Provides consistent logging for all operations
- State tracking across all UI operations
- Implements full UIManager trait

#### Enhanced UIEvent Types
- `WindowFocused` / `WindowBlurred` - Window focus events
- `NotificationActionClicked` - Notification action button clicks
- `TrayIconDoubleClicked` - Tray icon double-click
- `TrayMenuItemSelected` - Tray context menu selection
- `ThemeChanged` - Theme change events
- `ApplicationQuitting` - Application quit event

### Factory Enhancements (`ui-manager/src/lib.rs`)

#### New Factory Methods
- `available_frameworks()` - Get list of available UI frameworks on current platform
- `default_framework()` - Get recommended framework for current platform
- `is_framework_available()` - Check if a specific framework is available

#### UIConfiguration
- `current()` - Get current UI configuration
- `current_platform()` - Get current platform name
- Contains framework, availability, capabilities, and platform info

#### Factory Behavior
- All factory methods now wrap implementations in `UIManagerAdapter` for consistent logging
- Compile-time feature selection via Cargo features (flutter-ui, winui-ui, gtk-ui, qt-ui)
- Default to Flutter if no specific UI is selected

### Framework Implementations

All framework implementations (Flutter, WinUI, GTK, Qt) updated to implement the full interface:

#### Common Features Across All Implementations
- State tracking (window visibility, tray status, theme, hotkeys, event handlers)
- Configuration options for enabling/disabling features
- Proper initialization and shutdown lifecycle
- Platform-specific capability reporting

#### Flutter (`ui-manager/src/flutter.rs`)
- Added `THEME_CHANNEL` for theme management
- `initial_theme` configuration option
- Full state management with event handler support

#### WinUI (`ui-manager/src/native/winui.rs`)
- Windows-only implementation with stub for other platforms
- Jump Lists and Live Tiles capability support
- Windows-specific configuration options

#### GTK (`ui-manager/src/native/gtk.rs`)
- Linux-focused implementation
- Application ID configuration
- Cross-platform capability (Linux, Windows, macOS)

#### Qt (`ui-manager/src/native/qt.rs`)
- Cross-platform implementation
- Organization and application name configuration
- Jump Lists support on Windows

### Unit Tests (15 tests)
- Factory creation and framework selection
- Available frameworks detection
- Framework availability checking
- UI configuration retrieval
- Flutter manager creation, initialization, and state management
- Theme management
- Notification handling
- Hotkey registration and unregistration
- Window visibility and tray state

### Requirements Implemented
- **Requirement 4.1**: Flutter cross-platform UI and platform native UI options
- **Requirement 8.1**: WinUI 3 framework support for Windows native mode


## Cross-Platform System Integration (Task 10.4)

### System Integration Module (`ui-manager/src/system_integration.rs`)

A unified cross-platform system integration layer providing hotkey management, notifications, and system tray functionality.

#### CrossPlatformHotkeyManager - Unified Global Hotkey Management
- `initialize()` - Initialize the hotkey manager with platform-specific setup
- `register_hotkey()` - Register a global hotkey with callback
- `unregister_hotkey()` - Unregister a specific hotkey
- `unregister_all()` - Unregister all hotkeys
- `get_registered_hotkeys()` - Get list of registered hotkeys
- `is_hotkey_registered()` - Check if a hotkey is registered
- `parse_key_combination()` - Parse key combination strings (e.g., "Ctrl+Shift+F")
- Platform-specific registration stubs for Windows, Linux, and macOS
- `HotkeyRegistration` - Registration info with id, key combination, action, description, active status
- `HotkeyCallback` trait - Callback interface for hotkey events
- `FnHotkeyCallback` - Simple function-based callback implementation
- `ParsedKeyCombination` - Parsed key combination with modifiers and key
- `KeyModifier` - Modifier keys (Ctrl, Alt, Shift, Meta/Win/Cmd)

#### CrossPlatformNotificationManager - Unified System Notifications
- `initialize()` - Initialize notification support
- `show_notification()` - Show a notification with full configuration
- `show_simple()` - Show a simple notification with just a message
- `show_with_title()` - Show a notification with title and message
- `show_tab_activity()` - Pre-built notification for tab activity
- `show_bookmark_sync()` - Pre-built notification for bookmark sync completion
- `show_analysis_complete()` - Pre-built notification for analysis completion
- `show_duplicates_found()` - Pre-built notification for duplicate detection
- `get_history()` - Get notification history
- `clear_history()` - Clear notification history
- Platform-specific notification stubs for Windows (Toast), Linux (libnotify/D-Bus), macOS (UNUserNotificationCenter)
- `NotificationRecord` - Record of sent notifications with status tracking
- `NotificationStatus` - Status enum (Pending, Shown, Clicked, Dismissed, TimedOut, Failed)
- `NotificationCallback` trait - Callback interface for notification events

#### CrossPlatformTrayManager - Unified System Tray Functionality
- `initialize()` - Initialize system tray with default menu
- `set_menu()` - Set context menu items
- `set_tooltip()` - Update tooltip text
- `set_activity_badge()` - Update tooltip with activity count
- `set_event_handler()` - Set event handler for tray events
- `show()` / `hide()` - Show/hide tray icon
- `get_tooltip()` / `get_menu()` - Get current tooltip and menu
- Platform-specific tray stubs for Windows (Shell_NotifyIcon), Linux (StatusNotifierItem/AppIndicator), macOS (NSStatusItem)
- `TrayMenuItem` - Menu item types (Item, Separator, Submenu) with factory methods
- `TrayEvent` - Event types (IconClicked, IconDoubleClicked, IconRightClicked, MenuItemSelected)
- `TrayEventHandler` trait - Callback interface for tray events
- `FnTrayEventHandler` - Simple function-based event handler

#### SystemIntegrationService - Combined Service
- `initialize()` - Initialize all system integration components
- `is_initialized()` - Check initialization status
- Hotkey methods: `register_hotkey()`, `register_default_hotkeys()`, `unregister_hotkey()`, `unregister_all_hotkeys()`, `get_registered_hotkeys()`
- Notification methods: `show_notification()`, `show_simple_notification()`, `show_notification_with_title()`, `notify_tab_activity()`, `notify_bookmark_sync()`, `notify_analysis_complete()`, `notify_duplicates_found()`
- Tray methods: `show_tray()`, `hide_tray()`, `set_tray_tooltip()`, `set_tray_badge()`, `set_tray_menu()`, `set_tray_event_handler()`
- `shutdown()` - Shutdown all components
- Access to individual managers via `hotkey_manager()`, `notification_manager()`, `tray_manager()`

### Flutter UI Services Updates (`flutter_ui/lib/services/`)

#### HotkeyService (`hotkey_service.dart`)
- `HotkeyDefinition` - Hotkey definition with id, key combination, action, description
- `initialize()` - Initialize the hotkey service
- `registerHotkey()` - Register a hotkey with callback
- `unregisterHotkey()` - Unregister a hotkey
- `registerDefaultHotkeys()` - Register default application hotkeys (Ctrl+Shift+F, Ctrl+Shift+W, Ctrl+Shift+T)
- `unregisterAll()` - Unregister all hotkeys
- `handleHotkeyPressed()` - Handle hotkey press events from Rust backend
- `registeredHotkeyIds` / `registeredHotkeys` - Get registered hotkeys
- `isHotkeyRegistered()` - Check if a hotkey is registered

#### NotificationService (`notification_service.dart`)
- `NotificationConfig` - Notification configuration with title, body, urgency, actions, timeout
- `NotificationUrgency` - Urgency levels (low, normal, high, critical)
- `NotificationAction` - Action button definition
- `initialize()` - Initialize the notification service
- `showNotification()` - Show a notification with full configuration
- `showSimple()` / `showWithTitle()` - Convenience methods
- `showTabActivityNotification()` - Pre-built notification for tab activity
- `showBookmarkSyncNotification()` - Pre-built notification for bookmark sync
- `showAnalysisCompleteNotification()` - Pre-built notification for analysis completion
- `showDuplicatesFoundNotification()` - Pre-built notification for duplicate detection

#### SystemTrayService (`system_tray_service.dart`)
- `TrayMenuItem` - Abstract class with static factory methods (item, separator, submenu)
- `TrayMenuItemLabel` / `TrayMenuSeparator` / `TrayMenuSubmenu` - Menu item implementations
- `TrayEvent` / `TrayEventType` - Event types and event class
- `TrayEventCallback` - Callback type for tray events
- `initialize()` - Initialize with default menu
- `setMenu()` - Set context menu items
- `setTooltip()` - Update tooltip text
- `setActivityBadge()` - Update tooltip with activity count
- `setEventCallback()` - Set event callback
- `handleTrayEvent()` - Handle tray events from Rust backend
- `show()` / `hide()` - Show/hide tray icon
- `hideToTray()` / `restoreFromTray()` - Window management

### Unit Tests (12 tests)
- Hotkey manager creation and initialization
- Key combination parsing (Ctrl+Shift+F, Alt+Tab, Win+E)
- Hotkey registration and unregistration
- Notification manager creation and initialization
- Notification showing and history tracking
- Tray manager creation and initialization
- Tray menu item creation (item, separator, checkable)
- Tray tooltip management and activity badge
- System integration service lifecycle

### Requirements Implemented
- **Requirement 4.2**: System tray with quick access functionality (Windows, Linux, macOS)
- **Requirement 4.4**: Global hotkey support for quick operations


## WinUI 3 Implementation (Task 10.2)

### WinUI 3 Manager (`ui-manager/src/native/winui.rs`)

A comprehensive Windows-native UI implementation using WinUI 3 framework with Windows 11 design language.

#### Enhanced Configuration (`WinUIConfig`)
- `window_width/height` - Initial window dimensions
- `min_window_width/height` - Minimum window dimensions
- `enable_backdrop` - Mica/Acrylic backdrop support
- `enable_snap_layouts` - Windows 11 snap layouts support
- `enable_taskbar_progress` - Taskbar progress indicators
- `jump_list_max_items` - Maximum items in Jump List

#### Windows-Specific Features

**Jump Lists** - Quick taskbar access:
- `JumpListItem` - Item with id, title, description, icon, arguments, category
- `JumpListCategory` - Categories (Recent, Frequent, Tasks, Custom)
- `update_jump_list()` - Update Jump List with items
- `clear_jump_list()` - Clear all Jump List items
- `get_jump_list_items()` - Get current Jump List items

**Live Tiles** - Start menu tile updates:
- `LiveTileUpdate` - Update data with template, text lines, badge count, image
- `LiveTileTemplate` - Templates (TileSquareText01, TileSquareText02, TileSquareImage, TileWideText01, TileWideImage)
- `update_live_tile()` - Update Live Tile content
- `clear_live_tile()` - Clear Live Tile content
- `get_live_tile_state()` - Get current Live Tile state

**Toast Notifications** - Rich Windows notifications:
- `WindowsToastOptions` - Options for duration, audio, scenario, hero image, app logo, attribution
- `ToastDuration` - Duration options (Short ~7s, Long ~25s)
- `ToastAudio` - Audio options (Default, Silent, Custom, System)
- `ToastScenario` - Scenarios (Default, Alarm, Reminder, IncomingCall)
- `show_windows_toast()` - Show toast with rich options

**Taskbar Progress** - Progress indicators on taskbar:
- `TaskbarProgressState` - States (None, Indeterminate, Normal, Error, Paused)
- `set_taskbar_progress()` - Set progress state and value
- `get_taskbar_progress()` - Get current progress state

#### Window Management
- `set_window_position()` / `get_window_position()` - Position management
- `set_window_size()` / `get_window_size()` - Size management with minimum constraints
- `maximize_window()` / `restore_window()` - Maximize/restore functionality
- `is_maximized()` - Check maximized state
- `set_backdrop_enabled()` - Enable/disable Mica/Acrylic backdrop
- `flash_taskbar()` - Flash taskbar button for attention
- `add_to_recent_documents()` - Add to Windows recent documents

#### System Integration
- Automatic Jump List updates when UI data changes
- Automatic Live Tile updates with page/group counts
- Theme adaptation (Light/Dark/System) with Windows theme listener
- Global hotkey registration with Windows API
- System tray integration with context menu

#### Data Structures
- `WinUIState` - Extended state with window position, size, maximized status, backdrop status
- `WindowsSystemIntegration` - Windows-specific integration state (Jump List items, Live Tile state, taskbar progress)

#### Unit Tests (Windows-only, 11 tests)
- Manager creation and configuration
- Initialization and capabilities
- Window state management (show, hide, minimize to tray, restore)
- Theme management
- Jump List operations (update, clear)
- Live Tile operations (update, clear)
- Taskbar progress management
- Window position and size management
- Maximize/restore functionality
- Shutdown and cleanup

### Requirements Implemented
- **Requirement 8.1**: WinUI 3 framework with Windows 11 design language
- **Requirement 8.2**: Deep Windows integration (Jump Lists, Live Tiles)
- **Requirement 8.3**: Windows-specific shortcuts and gestures
- **Requirement 8.4**: Optimal startup performance and memory efficiency
