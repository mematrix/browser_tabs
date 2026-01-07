//! Data Access Layer for Web Page Manager
//! 
//! This module provides database operations and data persistence
//! using SQLite with FTS5 for full-text search.
//!
//! # Features
//! - SQLite database with FTS5 full-text search
//! - Schema migrations support
//! - LRU caching with TTL
//! - Repository pattern for data access
//! - Unified search across pages, history, and archives

pub mod schema;
pub mod repository;
pub mod cache;

pub use repository::*;
pub use cache::*;

use web_page_manager_core::*;
use std::path::Path;
use tokio_rusqlite::Connection;
use std::sync::Arc;
use tracing::{info, warn, debug};

/// Database manager for handling SQLite connections and migrations
pub struct DatabaseManager {
    connection: Arc<Connection>,
    cache: Arc<DataCache>,
}

impl DatabaseManager {
    /// Create a new database manager with the specified path
    pub async fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        Self::with_cache_config(db_path, CacheConfig::default()).await
    }

    /// Create a new database manager with custom cache configuration
    pub async fn with_cache_config<P: AsRef<Path>>(db_path: P, cache_config: CacheConfig) -> Result<Self> {
        let path = db_path.as_ref().to_path_buf();
        
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| WebPageManagerError::System {
                    source: SystemError::IO { source: e },
                })?;
            }
        }
        
        let connection = Connection::open(&path)
            .await
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to open database at {:?}: {}", path, e),
                },
            })?;
        
        let manager = Self {
            connection: Arc::new(connection),
            cache: Arc::new(DataCache::new(cache_config)),
        };
        
        // Run migrations
        manager.run_migrations().await?;
        
        info!("Database initialized at {:?}", path);
        
        Ok(manager)
    }

    /// Create an in-memory database (for testing)
    pub async fn in_memory() -> Result<Self> {
        Self::in_memory_with_cache(CacheConfig::default()).await
    }

    /// Create an in-memory database with custom cache configuration
    pub async fn in_memory_with_cache(cache_config: CacheConfig) -> Result<Self> {
        let connection = Connection::open(":memory:")
            .await
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to create in-memory database: {}", e),
                },
            })?;
        
        let manager = Self {
            connection: Arc::new(connection),
            cache: Arc::new(DataCache::new(cache_config)),
        };
        
        manager.run_migrations().await?;
        
        debug!("In-memory database initialized");
        
        Ok(manager)
    }

    /// Run database migrations
    async fn run_migrations(&self) -> Result<()> {
        // Get current schema version
        let current_version = self.get_schema_version().await?;
        let target_version = schema::SCHEMA_VERSION;
        
        if current_version >= target_version {
            debug!("Database schema is up to date (version {})", current_version);
            return Ok(());
        }
        
        info!("Migrating database from version {} to {}", current_version, target_version);
        
        // Run migrations in order
        for version in (current_version + 1)..=target_version {
            if let Some(migration) = schema::get_migration(version) {
                self.apply_migration(migration).await?;
                info!("Applied migration {}: {}", version, migration.description);
            } else {
                warn!("Migration {} not found, skipping", version);
            }
        }
        
        Ok(())
    }

    /// Get current schema version
    async fn get_schema_version(&self) -> Result<u32> {
        self.connection
            .call(|conn| {
                // Check if schema_migrations table exists
                let table_exists: bool = conn.query_row(
                    "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='schema_migrations'",
                    [],
                    |row| row.get(0),
                )?;
                
                if !table_exists {
                    return Ok(0);
                }
                
                // Get max version
                let version: Option<u32> = conn.query_row(
                    "SELECT MAX(version) FROM schema_migrations",
                    [],
                    |row| row.get(0),
                ).ok();
                
                Ok(version.unwrap_or(0))
            })
            .await
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to get schema version: {}", e),
                },
            })
    }

    /// Apply a single migration
    async fn apply_migration(&self, migration: &'static schema::Migration) -> Result<()> {
        let version = migration.version;
        let description = migration.description;
        let sql = migration.sql;
        
        self.connection
            .call(move |conn| {
                // Execute migration SQL
                conn.execute_batch(sql)?;
                
                // Record migration
                conn.execute(
                    "INSERT INTO schema_migrations (version, applied_at, description) VALUES (?1, ?2, ?3)",
                    rusqlite::params![version, Utc::now().timestamp(), description],
                )?;
                
                Ok(())
            })
            .await
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to apply migration {}: {}", version, e),
                },
            })?;
        
        Ok(())
    }

    /// Get the connection for repository operations
    pub fn connection(&self) -> Arc<Connection> {
        Arc::clone(&self.connection)
    }

    /// Get the cache instance
    pub fn cache(&self) -> Arc<DataCache> {
        Arc::clone(&self.cache)
    }

    /// Create a page repository
    pub fn page_repository(&self) -> SqlitePageRepository {
        SqlitePageRepository::new(self.connection())
    }

    /// Create a group repository
    pub fn group_repository(&self) -> SqliteGroupRepository {
        SqliteGroupRepository::new(self.connection())
    }

    /// Create a history repository
    pub fn history_repository(&self) -> SqliteHistoryRepository {
        SqliteHistoryRepository::new(self.connection())
    }

    /// Create an archive repository
    pub fn archive_repository(&self) -> SqliteArchiveRepository {
        SqliteArchiveRepository::new(self.connection())
    }

    /// Create a unified search repository
    pub fn unified_search_repository(&self) -> UnifiedSearchRepository {
        UnifiedSearchRepository::new(self.connection())
    }

    /// Get database statistics
    pub async fn stats(&self) -> Result<DatabaseStats> {
        let connection = self.connection();
        
        let (page_count, group_count, history_count, archive_count, db_size) = connection
            .call(|conn| {
                let page_count: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM unified_pages",
                    [],
                    |row| row.get(0),
                )?;
                
                let group_count: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM smart_groups",
                    [],
                    |row| row.get(0),
                )?;
                
                let history_count: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM tab_history",
                    [],
                    |row| row.get(0),
                )?;
                
                let archive_count: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM content_archives",
                    [],
                    |row| row.get(0),
                )?;
                
                let db_size: i64 = conn.query_row(
                    "SELECT page_count * page_size FROM pragma_page_count(), pragma_page_size()",
                    [],
                    |row| row.get(0),
                ).unwrap_or(0);
                
                Ok((page_count, group_count, history_count, archive_count, db_size))
            })
            .await
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to get database stats: {}", e),
                },
            })?;
        
        let cache_stats = self.cache.stats().await;
        
        Ok(DatabaseStats {
            page_count: page_count as usize,
            group_count: group_count as usize,
            history_count: history_count as usize,
            archive_count: archive_count as usize,
            database_size_bytes: db_size as u64,
            cache_stats,
        })
    }

    /// Optimize database (vacuum and analyze)
    pub async fn optimize(&self) -> Result<()> {
        self.connection
            .call(|conn| {
                conn.execute_batch("VACUUM; ANALYZE;")?;
                Ok(())
            })
            .await
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to optimize database: {}", e),
                },
            })?;
        
        info!("Database optimized");
        Ok(())
    }

    /// Clear all caches
    pub async fn clear_cache(&self) {
        self.cache.clear_all().await;
        debug!("Cache cleared");
    }
}

/// Database statistics
#[derive(Debug, Clone)]
pub struct DatabaseStats {
    pub page_count: usize,
    pub group_count: usize,
    pub history_count: usize,
    pub archive_count: usize,
    pub database_size_bytes: u64,
    pub cache_stats: CacheStats,
}

/// Cached page repository that uses the cache layer
pub struct CachedPageRepository {
    inner: SqlitePageRepository,
    cache: Arc<DataCache>,
}

impl CachedPageRepository {
    pub fn new(connection: Arc<Connection>, cache: Arc<DataCache>) -> Self {
        Self {
            inner: SqlitePageRepository::new(connection),
            cache,
        }
    }

    /// Save a page and update cache
    pub async fn save(&self, page: &UnifiedPageInfo) -> Result<()> {
        self.inner.save(page).await?;
        self.cache.cache_page(page).await;
        Ok(())
    }

    /// Get a page by ID, checking cache first
    pub async fn get_by_id(&self, id: &Uuid) -> Result<Option<UnifiedPageInfo>> {
        // Check cache first
        if let Some(page) = self.cache.get_page(id).await {
            return Ok(Some(page));
        }
        
        // Fetch from database
        let page = self.inner.get_by_id(id).await?;
        
        // Cache the result
        if let Some(ref p) = page {
            self.cache.cache_page(p).await;
        }
        
        Ok(page)
    }

    /// Get a page by URL, checking cache first
    pub async fn get_by_url(&self, url: &str) -> Result<Option<UnifiedPageInfo>> {
        // Check cache for URL -> ID mapping
        if let Some(id) = self.cache.get_page_id_by_url(url).await {
            if let Some(page) = self.cache.get_page(&id).await {
                return Ok(Some(page));
            }
        }
        
        // Fetch from database
        let page = self.inner.get_by_url(url).await?;
        
        // Cache the result
        if let Some(ref p) = page {
            self.cache.cache_page(p).await;
        }
        
        Ok(page)
    }

    /// Delete a page and invalidate cache
    pub async fn delete(&self, id: &Uuid) -> Result<()> {
        self.cache.invalidate_page(id).await;
        self.inner.delete(id).await
    }

    /// Search pages (not cached)
    pub async fn search(&self, query: &str) -> Result<Vec<UnifiedPageInfo>> {
        self.inner.search(query).await
    }

    /// Get all pages (not cached)
    pub async fn get_all(&self) -> Result<Vec<UnifiedPageInfo>> {
        self.inner.get_all().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_database_manager_in_memory() {
        let db = DatabaseManager::in_memory().await.unwrap();
        let stats = db.stats().await.unwrap();
        assert_eq!(stats.page_count, 0);
        assert_eq!(stats.group_count, 0);
    }

    #[tokio::test]
    async fn test_page_repository_crud() {
        let db = DatabaseManager::in_memory().await.unwrap();
        let repo = db.page_repository();
        
        let page = UnifiedPageInfo {
            id: Uuid::new_v4(),
            url: "https://example.com".to_string(),
            title: "Example Page".to_string(),
            favicon_url: None,
            content_summary: None,
            keywords: vec!["test".to_string(), "example".to_string()],
            category: Some("test".to_string()),
            source_type: PageSourceType::Bookmark {
                browser: BrowserType::Chrome,
                bookmark_id: BookmarkId::new(),
            },
            browser_info: None,
            tab_info: None,
            bookmark_info: None,
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            access_count: 0,
        };
        
        // Save
        repo.save(&page).await.unwrap();
        
        // Get by ID
        let fetched = repo.get_by_id(&page.id).await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().url, "https://example.com");
        
        // Get by URL
        let fetched_by_url = repo.get_by_url("https://example.com").await.unwrap();
        assert!(fetched_by_url.is_some());
        
        // Count
        let count = repo.count().await.unwrap();
        assert_eq!(count, 1);
        
        // Delete
        repo.delete(&page.id).await.unwrap();
        let deleted = repo.get_by_id(&page.id).await.unwrap();
        assert!(deleted.is_none());
    }

    #[tokio::test]
    async fn test_full_text_search() {
        let db = DatabaseManager::in_memory().await.unwrap();
        let repo = db.page_repository();
        
        let page1 = UnifiedPageInfo {
            id: Uuid::new_v4(),
            url: "https://rust-lang.org".to_string(),
            title: "Rust Programming Language".to_string(),
            favicon_url: None,
            content_summary: None,
            keywords: vec!["rust".to_string(), "programming".to_string()],
            category: Some("programming".to_string()),
            source_type: PageSourceType::Bookmark {
                browser: BrowserType::Chrome,
                bookmark_id: BookmarkId::new(),
            },
            browser_info: None,
            tab_info: None,
            bookmark_info: None,
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            access_count: 0,
        };
        
        let page2 = UnifiedPageInfo {
            id: Uuid::new_v4(),
            url: "https://python.org".to_string(),
            title: "Python Programming Language".to_string(),
            favicon_url: None,
            content_summary: None,
            keywords: vec!["python".to_string(), "programming".to_string()],
            category: Some("programming".to_string()),
            source_type: PageSourceType::Bookmark {
                browser: BrowserType::Firefox,
                bookmark_id: BookmarkId::new(),
            },
            browser_info: None,
            tab_info: None,
            bookmark_info: None,
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            access_count: 0,
        };
        
        repo.save(&page1).await.unwrap();
        repo.save(&page2).await.unwrap();
        
        // Search for "rust"
        let results = repo.search("rust").await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Rust Programming Language");
        
        // Search for "programming" - should find both
        let results = repo.search("programming").await.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_group_repository() {
        let db = DatabaseManager::in_memory().await.unwrap();
        let group_repo = db.group_repository();
        let page_repo = db.page_repository();
        
        // Create a page first
        let page = UnifiedPageInfo {
            id: Uuid::new_v4(),
            url: "https://example.com".to_string(),
            title: "Example".to_string(),
            favicon_url: None,
            content_summary: None,
            keywords: vec![],
            category: None,
            source_type: PageSourceType::Bookmark {
                browser: BrowserType::Chrome,
                bookmark_id: BookmarkId::new(),
            },
            browser_info: None,
            tab_info: None,
            bookmark_info: None,
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            access_count: 0,
        };
        page_repo.save(&page).await.unwrap();
        
        // Create a group
        let group = SmartGroup {
            id: Uuid::new_v4(),
            name: "Test Group".to_string(),
            description: "A test group".to_string(),
            group_type: GroupType::UserDefined,
            pages: vec![],
            created_at: Utc::now(),
            auto_generated: false,
            similarity_threshold: 0.8,
        };
        
        group_repo.save(&group).await.unwrap();
        
        // Add page to group
        group_repo.add_page_to_group(&page.id, &group.id, 0.9).await.unwrap();
        
        // Get pages in group
        let pages = group_repo.get_pages_in_group(&group.id).await.unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0], page.id);
        
        // Get groups for page
        let groups = group_repo.get_groups_for_page(&page.id).await.unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0], group.id);
    }

    #[tokio::test]
    async fn test_cached_page_repository() {
        let db = DatabaseManager::in_memory().await.unwrap();
        let cached_repo = CachedPageRepository::new(db.connection(), db.cache());
        
        let page = UnifiedPageInfo {
            id: Uuid::new_v4(),
            url: "https://cached-example.com".to_string(),
            title: "Cached Example".to_string(),
            favicon_url: None,
            content_summary: None,
            keywords: vec![],
            category: None,
            source_type: PageSourceType::Bookmark {
                browser: BrowserType::Chrome,
                bookmark_id: BookmarkId::new(),
            },
            browser_info: None,
            tab_info: None,
            bookmark_info: None,
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            access_count: 0,
        };
        
        // Save (should cache)
        cached_repo.save(&page).await.unwrap();
        
        // Get by ID (should hit cache)
        let fetched = cached_repo.get_by_id(&page.id).await.unwrap();
        assert!(fetched.is_some());
        
        // Get by URL (should hit cache)
        let fetched_by_url = cached_repo.get_by_url("https://cached-example.com").await.unwrap();
        assert!(fetched_by_url.is_some());
        
        // Check cache stats
        let stats = db.cache().stats().await;
        assert!(stats.pages_count > 0);
    }
}
