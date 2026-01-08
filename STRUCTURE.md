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
