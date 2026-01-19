// ! Batch operations for improved performance
//!
//! This module provides batch operations for database writes,
//! significantly improving performance when inserting multiple records.

use web_page_manager_core::*;
use tokio_rusqlite::Connection;
use std::sync::Arc;

/// Batch size for insert operations
const DEFAULT_BATCH_SIZE: usize = 100;

/// Batch page operations for improved performance
pub struct BatchPageOperations {
    connection: Arc<Connection>,
    batch_size: usize,
}

impl BatchPageOperations {
    pub fn new(connection: Arc<Connection>) -> Self {
        Self {
            connection,
            batch_size: DEFAULT_BATCH_SIZE,
        }
    }

    pub fn with_batch_size(connection: Arc<Connection>, batch_size: usize) -> Self {
        Self {
            connection,
            batch_size,
        }
    }

    /// Save multiple pages in a single transaction
    ///
    /// This is significantly faster than individual saves for large datasets
    pub async fn batch_save(&self, pages: &[UnifiedPageInfo]) -> Result<()> {
        if pages.is_empty() {
            return Ok(());
        }

        // Process in chunks to avoid extremely large transactions
        for chunk in pages.chunks(self.batch_size) {
            self.save_chunk(chunk).await?;
        }

        Ok(())
    }

    /// Save a chunk of pages in a single transaction
    async fn save_chunk(&self, pages: &[UnifiedPageInfo]) -> Result<()> {
        let pages_vec: Vec<UnifiedPageInfo> = pages.to_vec();

        self.connection
            .call(move |conn| {
                let tx = conn.transaction()?;

                {
                    let mut stmt = tx.prepare_cached(
                        r#"
                        INSERT OR REPLACE INTO unified_pages
                        (id, url, title, favicon_url, content_summary, keywords, category,
                         source_type, browser_info, tab_info, bookmark_info, created_at, last_accessed, access_count)
                        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
                        "#,
                    )?;

                    for page in &pages_vec {
                        let content_summary_json = page.content_summary
                            .as_ref()
                            .map(|s| serde_json::to_string(s).unwrap_or_default());
                        let keywords_json = serde_json::to_string(&page.keywords).unwrap_or_default();
                        let source_type_json = serde_json::to_string(&page.source_type).unwrap_or_default();
                        let browser_info_json = page.browser_info
                            .as_ref()
                            .map(|b| serde_json::to_string(b).unwrap_or_default());
                        let tab_info_json = page.tab_info
                            .as_ref()
                            .map(|t| serde_json::to_string(t).unwrap_or_default());
                        let bookmark_info_json = page.bookmark_info
                            .as_ref()
                            .map(|b| serde_json::to_string(b).unwrap_or_default());

                        stmt.execute(rusqlite::params![
                            page.id.to_string(),
                            page.url,
                            page.title,
                            page.favicon_url,
                            content_summary_json,
                            keywords_json,
                            page.category,
                            source_type_json,
                            browser_info_json,
                            tab_info_json,
                            bookmark_info_json,
                            page.created_at.timestamp(),
                            page.last_accessed.timestamp(),
                            page.access_count,
                        ])?;
                    }
                }

                tx.commit()?;
                Ok(())
            })
            .await
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to batch save pages: {}", e),
                },
            })?;

        Ok(())
    }

    /// Delete multiple pages in a single transaction
    pub async fn batch_delete(&self, ids: &[Uuid]) -> Result<usize> {
        if ids.is_empty() {
            return Ok(0);
        }

        let id_strings: Vec<String> = ids.iter().map(|id| id.to_string()).collect();

        let deleted = self.connection
            .call(move |conn| {
                let tx = conn.transaction()?;
                let mut total_deleted = 0;

                {
                    let mut stmt = tx.prepare_cached("DELETE FROM unified_pages WHERE id = ?1")?;

                    for id_str in &id_strings {
                        total_deleted += stmt.execute([id_str])?;
                    }
                }

                tx.commit()?;
                Ok(total_deleted)
            })
            .await
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to batch delete pages: {}", e),
                },
            })?;

        Ok(deleted)
    }

    /// Update access counts for multiple pages
    pub async fn batch_update_access(&self, ids: &[Uuid]) -> Result<()> {
        if ids.is_empty() {
            return Ok(());
        }

        let id_strings: Vec<String> = ids.iter().map(|id| id.to_string()).collect();
        let now = Utc::now().timestamp();

        self.connection
            .call(move |conn| {
                let tx = conn.transaction()?;

                {
                    let mut stmt = tx.prepare_cached(
                        "UPDATE unified_pages SET last_accessed = ?1, access_count = access_count + 1 WHERE id = ?2"
                    )?;

                    for id_str in &id_strings {
                        stmt.execute(rusqlite::params![now, id_str])?;
                    }
                }

                tx.commit()?;
                Ok(())
            })
            .await
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to batch update access: {}", e),
                },
            })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DatabaseManager, PageRepository};

    #[tokio::test]
    async fn test_batch_save_performance() {
        let db = DatabaseManager::in_memory().await.unwrap();
        let batch_ops = BatchPageOperations::new(db.connection());

        // Create 100 test pages
        let pages: Vec<UnifiedPageInfo> = (0..100)
            .map(|i| UnifiedPageInfo {
                id: Uuid::new_v4(),
                url: format!("https://example{}.com", i),
                title: format!("Test Page {}", i),
                favicon_url: None,
                content_summary: None,
                keywords: vec![format!("tag{}", i % 5)],
                category: Some(format!("category{}", i % 3)),
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
            })
            .collect();

        // Batch save
        let start = std::time::Instant::now();
        batch_ops.batch_save(&pages).await.unwrap();
        let duration = start.elapsed();

        println!("Batch saved 100 pages in {:?}", duration);

        // Verify all pages were saved
        let page_repo = db.page_repository();
        let count = page_repo.count().await.unwrap();
        assert_eq!(count, 100);
    }

    #[tokio::test]
    async fn test_batch_delete() {
        let db = DatabaseManager::in_memory().await.unwrap();
        let batch_ops = BatchPageOperations::new(db.connection());
        let page_repo = db.page_repository();

        // Create and save pages
        let pages: Vec<UnifiedPageInfo> = (0..10)
            .map(|i| UnifiedPageInfo {
                id: Uuid::new_v4(),
                url: format!("https://delete{}.com", i),
                title: format!("Delete Test {}", i),
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
            })
            .collect();

        let ids: Vec<Uuid> = pages.iter().map(|p| p.id).collect();

        batch_ops.batch_save(&pages).await.unwrap();

        // Verify pages exist
        let count_before = page_repo.count().await.unwrap();
        assert_eq!(count_before, 10);

        // Batch delete
        let deleted = batch_ops.batch_delete(&ids).await.unwrap();
        assert_eq!(deleted, 10);

        // Verify pages are deleted
        let count_after = page_repo.count().await.unwrap();
        assert_eq!(count_after, 0);
    }

    #[tokio::test]
    async fn test_batch_update_access() {
        let db = DatabaseManager::in_memory().await.unwrap();
        let batch_ops = BatchPageOperations::new(db.connection());
        let page_repo = db.page_repository();

        // Create and save pages
        let pages: Vec<UnifiedPageInfo> = (0..5)
            .map(|i| UnifiedPageInfo {
                id: Uuid::new_v4(),
                url: format!("https://access{}.com", i),
                title: format!("Access Test {}", i),
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
            })
            .collect();

        let ids: Vec<Uuid> = pages.iter().map(|p| p.id).collect();

        batch_ops.batch_save(&pages).await.unwrap();

        // Batch update access
        batch_ops.batch_update_access(&ids).await.unwrap();

        // Verify access counts increased
        for id in &ids {
            let page = page_repo.get_by_id(id).await.unwrap().unwrap();
            assert_eq!(page.access_count, 1);
        }
    }
}
