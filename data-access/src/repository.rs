//! Repository implementations for data access

use web_page_manager_core::*;
use tokio_rusqlite::Connection;
use std::sync::Arc;
use async_trait::async_trait;
use rusqlite::Row;

/// Repository trait for unified pages
#[async_trait]
pub trait PageRepository: Send + Sync {
    async fn save(&self, page: &UnifiedPageInfo) -> Result<()>;
    async fn get_by_id(&self, id: &Uuid) -> Result<Option<UnifiedPageInfo>>;
    async fn get_by_url(&self, url: &str) -> Result<Option<UnifiedPageInfo>>;
    async fn get_all(&self) -> Result<Vec<UnifiedPageInfo>>;
    async fn get_paginated(&self, limit: usize, offset: usize) -> Result<Vec<UnifiedPageInfo>>;
    async fn delete(&self, id: &Uuid) -> Result<()>;
    async fn search(&self, query: &str) -> Result<Vec<UnifiedPageInfo>>;
    async fn search_with_limit(&self, query: &str, limit: usize) -> Result<Vec<UnifiedPageInfo>>;
    async fn update_access(&self, id: &Uuid) -> Result<()>;
    async fn count(&self) -> Result<usize>;
}

/// Repository trait for smart groups
#[async_trait]
pub trait GroupRepository: Send + Sync {
    async fn save(&self, group: &SmartGroup) -> Result<()>;
    async fn get_by_id(&self, id: &Uuid) -> Result<Option<SmartGroup>>;
    async fn get_all(&self) -> Result<Vec<SmartGroup>>;
    async fn delete(&self, id: &Uuid) -> Result<()>;
    async fn add_page_to_group(&self, page_id: &Uuid, group_id: &Uuid, confidence: f32) -> Result<()>;
    async fn remove_page_from_group(&self, page_id: &Uuid, group_id: &Uuid) -> Result<()>;
    async fn get_pages_in_group(&self, group_id: &Uuid) -> Result<Vec<Uuid>>;
    async fn get_groups_for_page(&self, page_id: &Uuid) -> Result<Vec<Uuid>>;
}

/// Repository trait for tab history
#[async_trait]
pub trait HistoryRepository: Send + Sync {
    async fn save(&self, entry: &HistoryEntry) -> Result<()>;
    async fn get_by_id(&self, id: &HistoryId) -> Result<Option<HistoryEntry>>;
    async fn get_filtered(&self, filter: &HistoryFilter) -> Result<Vec<HistoryEntry>>;
    async fn delete(&self, id: &HistoryId) -> Result<()>;
    async fn delete_older_than(&self, timestamp: DateTime<Utc>) -> Result<usize>;
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<HistoryEntry>>;
    async fn count(&self) -> Result<usize>;
}

/// Repository trait for content archives
#[async_trait]
pub trait ArchiveRepository: Send + Sync {
    async fn save(&self, archive: &ContentArchive) -> Result<()>;
    async fn get_by_id(&self, id: &ArchiveId) -> Result<Option<ContentArchive>>;
    async fn get_by_page_id(&self, page_id: &Uuid) -> Result<Option<ContentArchive>>;
    async fn delete(&self, id: &ArchiveId) -> Result<()>;
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<ContentArchive>>;
    async fn get_total_size(&self) -> Result<u64>;
}

/// Content archive data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentArchive {
    pub id: ArchiveId,
    pub page_id: Uuid,
    pub url: String,
    pub title: String,
    pub content_html: String,
    pub content_text: String,
    pub media_files: Vec<String>,
    pub archived_at: DateTime<Utc>,
    pub file_size: u64,
    pub checksum: Option<String>,
}

/// Helper function to map a row to UnifiedPageInfo
fn row_to_page(row: &Row) -> rusqlite::Result<UnifiedPageInfo> {
    let id_str: String = row.get(0)?;
    let url: String = row.get(1)?;
    let title: String = row.get(2)?;
    let favicon_url: Option<String> = row.get(3)?;
    let content_summary_json: Option<String> = row.get(4)?;
    let keywords_json: String = row.get(5)?;
    let category: Option<String> = row.get(6)?;
    let source_type_json: String = row.get(7)?;
    let browser_info_json: Option<String> = row.get(8)?;
    let tab_info_json: Option<String> = row.get(9)?;
    let bookmark_info_json: Option<String> = row.get(10)?;
    let created_at_ts: i64 = row.get(11)?;
    let last_accessed_ts: i64 = row.get(12)?;
    let access_count: u32 = row.get(13)?;

    let id = Uuid::parse_str(&id_str).unwrap_or_else(|_| Uuid::new_v4());
    let content_summary = content_summary_json
        .and_then(|s| serde_json::from_str(&s).ok());
    let keywords: Vec<String> = serde_json::from_str(&keywords_json).unwrap_or_default();
    let source_type: PageSourceType = serde_json::from_str(&source_type_json)
        .unwrap_or(PageSourceType::Bookmark {
            browser: BrowserType::Chrome,
            bookmark_id: BookmarkId::new(),
        });
    let browser_info = browser_info_json
        .and_then(|s| serde_json::from_str(&s).ok());
    let tab_info = tab_info_json
        .and_then(|s| serde_json::from_str(&s).ok());
    let bookmark_info = bookmark_info_json
        .and_then(|s| serde_json::from_str(&s).ok());

    Ok(UnifiedPageInfo {
        id,
        url,
        title,
        favicon_url,
        content_summary,
        keywords,
        category,
        source_type,
        browser_info,
        tab_info,
        bookmark_info,
        created_at: DateTime::from_timestamp(created_at_ts, 0).unwrap_or_else(Utc::now),
        last_accessed: DateTime::from_timestamp(last_accessed_ts, 0).unwrap_or_else(Utc::now),
        access_count,
    })
}

/// Helper function to map a row to SmartGroup
fn row_to_group(row: &Row) -> rusqlite::Result<SmartGroup> {
    let id_str: String = row.get(0)?;
    let name: String = row.get(1)?;
    let description: Option<String> = row.get(2)?;
    let group_type_json: String = row.get(3)?;
    let created_at_ts: i64 = row.get(4)?;
    let auto_generated: bool = row.get(5)?;
    let similarity_threshold: f32 = row.get(6)?;

    let id = Uuid::parse_str(&id_str).unwrap_or_else(|_| Uuid::new_v4());
    let group_type: GroupType = serde_json::from_str(&group_type_json)
        .unwrap_or(GroupType::UserDefined);

    Ok(SmartGroup {
        id,
        name,
        description: description.unwrap_or_default(),
        group_type,
        pages: vec![], // Pages are loaded separately
        created_at: DateTime::from_timestamp(created_at_ts, 0).unwrap_or_else(Utc::now),
        auto_generated,
        similarity_threshold,
    })
}

/// SQLite implementation of PageRepository
pub struct SqlitePageRepository {
    connection: Arc<Connection>,
}

impl SqlitePageRepository {
    pub fn new(connection: Arc<Connection>) -> Self {
        Self { connection }
    }
}

#[async_trait]
impl PageRepository for SqlitePageRepository {
    async fn save(&self, page: &UnifiedPageInfo) -> Result<()> {
        let page_clone = page.clone();
        
        self.connection
            .call(move |conn| {
                let content_summary_json = page_clone.content_summary
                    .as_ref()
                    .map(|s| serde_json::to_string(s).unwrap_or_default());
                let keywords_json = serde_json::to_string(&page_clone.keywords).unwrap_or_default();
                let source_type_json = serde_json::to_string(&page_clone.source_type).unwrap_or_default();
                let browser_info_json = page_clone.browser_info
                    .as_ref()
                    .map(|b| serde_json::to_string(b).unwrap_or_default());
                let tab_info_json = page_clone.tab_info
                    .as_ref()
                    .map(|t| serde_json::to_string(t).unwrap_or_default());
                let bookmark_info_json = page_clone.bookmark_info
                    .as_ref()
                    .map(|b| serde_json::to_string(b).unwrap_or_default());
                
                conn.execute(
                    r#"
                    INSERT OR REPLACE INTO unified_pages 
                    (id, url, title, favicon_url, content_summary, keywords, category, 
                     source_type, browser_info, tab_info, bookmark_info, created_at, last_accessed, access_count)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
                    "#,
                    rusqlite::params![
                        page_clone.id.to_string(),
                        page_clone.url,
                        page_clone.title,
                        page_clone.favicon_url,
                        content_summary_json,
                        keywords_json,
                        page_clone.category,
                        source_type_json,
                        browser_info_json,
                        tab_info_json,
                        bookmark_info_json,
                        page_clone.created_at.timestamp(),
                        page_clone.last_accessed.timestamp(),
                        page_clone.access_count,
                    ],
                )?;
                Ok(())
            })
            .await
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to save page: {}", e),
                },
            })?;
        
        Ok(())
    }

    async fn get_by_id(&self, id: &Uuid) -> Result<Option<UnifiedPageInfo>> {
        let id_str = id.to_string();
        
        self.connection
            .call(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, url, title, favicon_url, content_summary, keywords, category, \
                     source_type, browser_info, tab_info, bookmark_info, created_at, last_accessed, access_count \
                     FROM unified_pages WHERE id = ?1"
                )?;
                
                let result = stmt.query_row([&id_str], row_to_page);
                
                match result {
                    Ok(page) => Ok(Some(page)),
                    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                    Err(e) => Err(e.into()),
                }
            })
            .await
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to get page: {}", e),
                },
            })
    }

    async fn get_by_url(&self, url: &str) -> Result<Option<UnifiedPageInfo>> {
        let url_str = url.to_string();
        
        self.connection
            .call(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, url, title, favicon_url, content_summary, keywords, category, \
                     source_type, browser_info, tab_info, bookmark_info, created_at, last_accessed, access_count \
                     FROM unified_pages WHERE url = ?1 LIMIT 1"
                )?;
                
                let result = stmt.query_row([&url_str], row_to_page);
                
                match result {
                    Ok(page) => Ok(Some(page)),
                    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                    Err(e) => Err(e.into()),
                }
            })
            .await
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to get page by URL: {}", e),
                },
            })
    }

    async fn get_all(&self) -> Result<Vec<UnifiedPageInfo>> {
        self.connection
            .call(|conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, url, title, favicon_url, content_summary, keywords, category, \
                     source_type, browser_info, tab_info, bookmark_info, created_at, last_accessed, access_count \
                     FROM unified_pages ORDER BY last_accessed DESC"
                )?;
                
                let rows = stmt.query_map([], row_to_page)?;
                let mut pages = Vec::new();
                for row in rows {
                    if let Ok(page) = row {
                        pages.push(page);
                    }
                }
                Ok(pages)
            })
            .await
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to get all pages: {}", e),
                },
            })
    }

    async fn get_paginated(&self, limit: usize, offset: usize) -> Result<Vec<UnifiedPageInfo>> {
        self.connection
            .call(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, url, title, favicon_url, content_summary, keywords, category, \
                     source_type, browser_info, tab_info, bookmark_info, created_at, last_accessed, access_count \
                     FROM unified_pages ORDER BY last_accessed DESC LIMIT ?1 OFFSET ?2"
                )?;
                
                let rows = stmt.query_map(rusqlite::params![limit as i64, offset as i64], row_to_page)?;
                let mut pages = Vec::new();
                for row in rows {
                    if let Ok(page) = row {
                        pages.push(page);
                    }
                }
                Ok(pages)
            })
            .await
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to get paginated pages: {}", e),
                },
            })
    }

    async fn delete(&self, id: &Uuid) -> Result<()> {
        let id_str = id.to_string();
        
        self.connection
            .call(move |conn| {
                conn.execute(
                    "DELETE FROM unified_pages WHERE id = ?1",
                    [&id_str],
                )?;
                Ok(())
            })
            .await
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to delete page: {}", e),
                },
            })?;
        
        Ok(())
    }

    async fn search(&self, query: &str) -> Result<Vec<UnifiedPageInfo>> {
        self.search_with_limit(query, 100).await
    }

    async fn search_with_limit(&self, query: &str, limit: usize) -> Result<Vec<UnifiedPageInfo>> {
        let query_str = format!("{}*", query.replace('"', "\"\""));
        
        self.connection
            .call(move |conn| {
                let mut stmt = conn.prepare(
                    r#"
                    SELECT p.id, p.url, p.title, p.favicon_url, p.content_summary, p.keywords, p.category,
                           p.source_type, p.browser_info, p.tab_info, p.bookmark_info, p.created_at, p.last_accessed, p.access_count
                    FROM unified_pages p
                    JOIN pages_fts fts ON p.rowid = fts.rowid
                    WHERE pages_fts MATCH ?1
                    ORDER BY rank
                    LIMIT ?2
                    "#
                )?;
                
                let rows = stmt.query_map(rusqlite::params![query_str, limit as i64], row_to_page)?;
                let mut pages = Vec::new();
                for row in rows {
                    if let Ok(page) = row {
                        pages.push(page);
                    }
                }
                Ok(pages)
            })
            .await
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to search pages: {}", e),
                },
            })
    }

    async fn update_access(&self, id: &Uuid) -> Result<()> {
        let id_str = id.to_string();
        let now = Utc::now().timestamp();
        
        self.connection
            .call(move |conn| {
                conn.execute(
                    "UPDATE unified_pages SET last_accessed = ?1, access_count = access_count + 1 WHERE id = ?2",
                    rusqlite::params![now, id_str],
                )?;
                Ok(())
            })
            .await
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to update page access: {}", e),
                },
            })?;
        
        Ok(())
    }

    async fn count(&self) -> Result<usize> {
        self.connection
            .call(|conn| {
                let count: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM unified_pages",
                    [],
                    |row| row.get(0),
                )?;
                Ok(count as usize)
            })
            .await
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to count pages: {}", e),
                },
            })
    }
}


/// SQLite implementation of GroupRepository
pub struct SqliteGroupRepository {
    connection: Arc<Connection>,
}

impl SqliteGroupRepository {
    pub fn new(connection: Arc<Connection>) -> Self {
        Self { connection }
    }
}

#[async_trait]
impl GroupRepository for SqliteGroupRepository {
    async fn save(&self, group: &SmartGroup) -> Result<()> {
        let group_clone = group.clone();
        
        self.connection
            .call(move |conn| {
                let group_type_json = serde_json::to_string(&group_clone.group_type).unwrap_or_default();
                
                conn.execute(
                    r#"
                    INSERT OR REPLACE INTO smart_groups 
                    (id, name, description, group_type, created_at, auto_generated, similarity_threshold)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                    "#,
                    rusqlite::params![
                        group_clone.id.to_string(),
                        group_clone.name,
                        group_clone.description,
                        group_type_json,
                        group_clone.created_at.timestamp(),
                        group_clone.auto_generated,
                        group_clone.similarity_threshold,
                    ],
                )?;
                Ok(())
            })
            .await
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to save group: {}", e),
                },
            })?;
        
        Ok(())
    }

    async fn get_by_id(&self, id: &Uuid) -> Result<Option<SmartGroup>> {
        let id_str = id.to_string();
        
        self.connection
            .call(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, name, description, group_type, created_at, auto_generated, similarity_threshold \
                     FROM smart_groups WHERE id = ?1"
                )?;
                
                let result = stmt.query_row([&id_str], row_to_group);
                
                match result {
                    Ok(group) => Ok(Some(group)),
                    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                    Err(e) => Err(e.into()),
                }
            })
            .await
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to get group: {}", e),
                },
            })
    }

    async fn get_all(&self) -> Result<Vec<SmartGroup>> {
        self.connection
            .call(|conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, name, description, group_type, created_at, auto_generated, similarity_threshold \
                     FROM smart_groups ORDER BY created_at DESC"
                )?;
                
                let rows = stmt.query_map([], row_to_group)?;
                let mut groups = Vec::new();
                for row in rows {
                    if let Ok(group) = row {
                        groups.push(group);
                    }
                }
                Ok(groups)
            })
            .await
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to get all groups: {}", e),
                },
            })
    }

    async fn delete(&self, id: &Uuid) -> Result<()> {
        let id_str = id.to_string();
        
        self.connection
            .call(move |conn| {
                conn.execute(
                    "DELETE FROM smart_groups WHERE id = ?1",
                    [&id_str],
                )?;
                Ok(())
            })
            .await
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to delete group: {}", e),
                },
            })?;
        
        Ok(())
    }

    async fn add_page_to_group(&self, page_id: &Uuid, group_id: &Uuid, confidence: f32) -> Result<()> {
        let page_id_str = page_id.to_string();
        let group_id_str = group_id.to_string();
        
        self.connection
            .call(move |conn| {
                conn.execute(
                    r#"
                    INSERT OR REPLACE INTO page_group_relations 
                    (page_id, group_id, added_at, confidence_score)
                    VALUES (?1, ?2, ?3, ?4)
                    "#,
                    rusqlite::params![
                        page_id_str,
                        group_id_str,
                        Utc::now().timestamp(),
                        confidence,
                    ],
                )?;
                Ok(())
            })
            .await
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to add page to group: {}", e),
                },
            })?;
        
        Ok(())
    }

    async fn remove_page_from_group(&self, page_id: &Uuid, group_id: &Uuid) -> Result<()> {
        let page_id_str = page_id.to_string();
        let group_id_str = group_id.to_string();
        
        self.connection
            .call(move |conn| {
                conn.execute(
                    "DELETE FROM page_group_relations WHERE page_id = ?1 AND group_id = ?2",
                    [&page_id_str, &group_id_str],
                )?;
                Ok(())
            })
            .await
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to remove page from group: {}", e),
                },
            })?;
        
        Ok(())
    }

    async fn get_pages_in_group(&self, group_id: &Uuid) -> Result<Vec<Uuid>> {
        let group_id_str = group_id.to_string();
        
        self.connection
            .call(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT page_id FROM page_group_relations WHERE group_id = ?1 ORDER BY confidence_score DESC"
                )?;
                
                let rows = stmt.query_map([&group_id_str], |row| {
                    let id_str: String = row.get(0)?;
                    Ok(Uuid::parse_str(&id_str).unwrap_or_else(|_| Uuid::new_v4()))
                })?;
                
                let mut page_ids = Vec::new();
                for row in rows {
                    if let Ok(id) = row {
                        page_ids.push(id);
                    }
                }
                Ok(page_ids)
            })
            .await
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to get pages in group: {}", e),
                },
            })
    }

    async fn get_groups_for_page(&self, page_id: &Uuid) -> Result<Vec<Uuid>> {
        let page_id_str = page_id.to_string();
        
        self.connection
            .call(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT group_id FROM page_group_relations WHERE page_id = ?1"
                )?;
                
                let rows = stmt.query_map([&page_id_str], |row| {
                    let id_str: String = row.get(0)?;
                    Ok(Uuid::parse_str(&id_str).unwrap_or_else(|_| Uuid::new_v4()))
                })?;
                
                let mut group_ids = Vec::new();
                for row in rows {
                    if let Ok(id) = row {
                        group_ids.push(id);
                    }
                }
                Ok(group_ids)
            })
            .await
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to get groups for page: {}", e),
                },
            })
    }
}


/// SQLite implementation of HistoryRepository
pub struct SqliteHistoryRepository {
    connection: Arc<Connection>,
}

impl SqliteHistoryRepository {
    pub fn new(connection: Arc<Connection>) -> Self {
        Self { connection }
    }
}

#[async_trait]
impl HistoryRepository for SqliteHistoryRepository {
    async fn save(&self, entry: &HistoryEntry) -> Result<()> {
        let entry_clone = entry.clone();
        
        self.connection
            .call(move |conn| {
                let session_info_json = entry_clone.session_info
                    .as_ref()
                    .map(|s| serde_json::to_string(s).unwrap_or_default());
                let content_summary_json = entry_clone.page_info.content_summary
                    .as_ref()
                    .map(|s| serde_json::to_string(s).unwrap_or_default());
                let tab_id_str = entry_clone.tab_id.as_ref().map(|t| t.0.clone());
                
                conn.execute(
                    r#"
                    INSERT OR REPLACE INTO tab_history 
                    (id, page_id, url, title, favicon_url, browser_type, tab_id, closed_at, session_info, content_summary)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                    "#,
                    rusqlite::params![
                        entry_clone.id.0.to_string(),
                        entry_clone.page_info.id.to_string(),
                        entry_clone.page_info.url,
                        entry_clone.page_info.title,
                        entry_clone.page_info.favicon_url,
                        serde_json::to_string(&entry_clone.browser_type).unwrap_or_default(),
                        tab_id_str,
                        entry_clone.closed_at.timestamp(),
                        session_info_json,
                        content_summary_json,
                    ],
                )?;
                Ok(())
            })
            .await
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to save history entry: {}", e),
                },
            })?;
        
        Ok(())
    }

    async fn get_by_id(&self, id: &HistoryId) -> Result<Option<HistoryEntry>> {
        let id_str = id.0.to_string();
        
        self.connection
            .call(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, page_id, url, title, favicon_url, browser_type, tab_id, closed_at, session_info, content_summary \
                     FROM tab_history WHERE id = ?1"
                )?;
                
                let result = stmt.query_row([&id_str], |row| {
                    row_to_history_entry(row)
                });
                
                match result {
                    Ok(entry) => Ok(Some(entry)),
                    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                    Err(e) => Err(e.into()),
                }
            })
            .await
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to get history entry: {}", e),
                },
            })
    }

    async fn get_filtered(&self, filter: &HistoryFilter) -> Result<Vec<HistoryEntry>> {
        let filter_clone = filter.clone();
        
        self.connection
            .call(move |conn| {
                let mut sql = String::from(
                    "SELECT id, page_id, url, title, favicon_url, browser_type, tab_id, closed_at, session_info, content_summary \
                     FROM tab_history WHERE 1=1"
                );
                let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
                
                if let Some(ref browser) = filter_clone.browser_type {
                    sql.push_str(" AND browser_type = ?");
                    params.push(Box::new(serde_json::to_string(browser).unwrap_or_default()));
                }
                
                if let Some(from) = filter_clone.from_date {
                    sql.push_str(" AND closed_at >= ?");
                    params.push(Box::new(from.timestamp()));
                }
                
                if let Some(to) = filter_clone.to_date {
                    sql.push_str(" AND closed_at <= ?");
                    params.push(Box::new(to.timestamp()));
                }
                
                if let Some(ref url_pattern) = filter_clone.url_pattern {
                    sql.push_str(" AND url LIKE ?");
                    params.push(Box::new(format!("%{}%", url_pattern)));
                }
                
                if let Some(ref title_pattern) = filter_clone.title_pattern {
                    sql.push_str(" AND title LIKE ?");
                    params.push(Box::new(format!("%{}%", title_pattern)));
                }
                
                sql.push_str(" ORDER BY closed_at DESC");
                
                if let Some(limit) = filter_clone.limit {
                    sql.push_str(&format!(" LIMIT {}", limit));
                }
                
                if let Some(offset) = filter_clone.offset {
                    sql.push_str(&format!(" OFFSET {}", offset));
                }
                
                let mut stmt = conn.prepare(&sql)?;
                let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
                
                let rows = stmt.query_map(param_refs.as_slice(), row_to_history_entry)?;
                let mut entries = Vec::new();
                for row in rows {
                    if let Ok(entry) = row {
                        entries.push(entry);
                    }
                }
                Ok(entries)
            })
            .await
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to get filtered history: {}", e),
                },
            })
    }

    async fn delete(&self, id: &HistoryId) -> Result<()> {
        let id_str = id.0.to_string();
        
        self.connection
            .call(move |conn| {
                conn.execute(
                    "DELETE FROM tab_history WHERE id = ?1",
                    [&id_str],
                )?;
                Ok(())
            })
            .await
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to delete history entry: {}", e),
                },
            })?;
        
        Ok(())
    }

    async fn delete_older_than(&self, timestamp: DateTime<Utc>) -> Result<usize> {
        let ts = timestamp.timestamp();
        
        self.connection
            .call(move |conn| {
                let deleted = conn.execute(
                    "DELETE FROM tab_history WHERE closed_at < ?1",
                    [ts],
                )?;
                Ok(deleted)
            })
            .await
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to delete old history: {}", e),
                },
            })
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<HistoryEntry>> {
        let query_str = format!("{}*", query.replace('"', "\"\""));
        
        self.connection
            .call(move |conn| {
                let mut stmt = conn.prepare(
                    r#"
                    SELECT h.id, h.page_id, h.url, h.title, h.favicon_url, h.browser_type, h.tab_id, 
                           h.closed_at, h.session_info, h.content_summary
                    FROM tab_history h
                    JOIN history_fts fts ON h.rowid = fts.rowid
                    WHERE history_fts MATCH ?1
                    ORDER BY rank
                    LIMIT ?2
                    "#
                )?;
                
                let rows = stmt.query_map(rusqlite::params![query_str, limit as i64], row_to_history_entry)?;
                let mut entries = Vec::new();
                for row in rows {
                    if let Ok(entry) = row {
                        entries.push(entry);
                    }
                }
                Ok(entries)
            })
            .await
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to search history: {}", e),
                },
            })
    }

    async fn count(&self) -> Result<usize> {
        self.connection
            .call(|conn| {
                let count: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM tab_history",
                    [],
                    |row| row.get(0),
                )?;
                Ok(count as usize)
            })
            .await
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to count history: {}", e),
                },
            })
    }
}

/// Helper function to map a row to HistoryEntry
fn row_to_history_entry(row: &Row) -> rusqlite::Result<HistoryEntry> {
    let id_str: String = row.get(0)?;
    let page_id_str: Option<String> = row.get(1)?;
    let url: String = row.get(2)?;
    let title: String = row.get(3)?;
    let favicon_url: Option<String> = row.get(4)?;
    let browser_type_json: String = row.get(5)?;
    let tab_id_str: Option<String> = row.get(6)?;
    let closed_at_ts: i64 = row.get(7)?;
    let session_info_json: Option<String> = row.get(8)?;
    let content_summary_json: Option<String> = row.get(9)?;

    let id = HistoryId(Uuid::parse_str(&id_str).unwrap_or_else(|_| Uuid::new_v4()));
    let page_id = page_id_str
        .and_then(|s| Uuid::parse_str(&s).ok())
        .unwrap_or_else(Uuid::new_v4);
    let browser_type: BrowserType = serde_json::from_str(&browser_type_json)
        .unwrap_or(BrowserType::Chrome);
    let tab_id = tab_id_str.map(TabId);
    let session_info = session_info_json
        .and_then(|s| serde_json::from_str(&s).ok());
    let content_summary = content_summary_json
        .and_then(|s| serde_json::from_str(&s).ok());

    let page_info = UnifiedPageInfo {
        id: page_id,
        url,
        title,
        favicon_url,
        content_summary,
        keywords: vec![],
        category: None,
        source_type: PageSourceType::ClosedTab { history_id: id.clone() },
        browser_info: None,
        tab_info: None,
        bookmark_info: None,
        created_at: DateTime::from_timestamp(closed_at_ts, 0).unwrap_or_else(Utc::now),
        last_accessed: DateTime::from_timestamp(closed_at_ts, 0).unwrap_or_else(Utc::now),
        access_count: 0,
    };

    Ok(HistoryEntry {
        id,
        page_info,
        browser_type,
        tab_id,
        closed_at: DateTime::from_timestamp(closed_at_ts, 0).unwrap_or_else(Utc::now),
        session_info,
    })
}


/// SQLite implementation of ArchiveRepository
pub struct SqliteArchiveRepository {
    connection: Arc<Connection>,
}

impl SqliteArchiveRepository {
    pub fn new(connection: Arc<Connection>) -> Self {
        Self { connection }
    }
}

#[async_trait]
impl ArchiveRepository for SqliteArchiveRepository {
    async fn save(&self, archive: &ContentArchive) -> Result<()> {
        let archive_clone = archive.clone();
        
        self.connection
            .call(move |conn| {
                let media_files_json = serde_json::to_string(&archive_clone.media_files).unwrap_or_default();
                
                conn.execute(
                    r#"
                    INSERT OR REPLACE INTO content_archives 
                    (id, page_id, url, title, content_html, content_text, media_files, archived_at, file_size, checksum)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                    "#,
                    rusqlite::params![
                        archive_clone.id.0.to_string(),
                        archive_clone.page_id.to_string(),
                        archive_clone.url,
                        archive_clone.title,
                        archive_clone.content_html,
                        archive_clone.content_text,
                        media_files_json,
                        archive_clone.archived_at.timestamp(),
                        archive_clone.file_size as i64,
                        archive_clone.checksum,
                    ],
                )?;
                Ok(())
            })
            .await
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to save archive: {}", e),
                },
            })?;
        
        Ok(())
    }

    async fn get_by_id(&self, id: &ArchiveId) -> Result<Option<ContentArchive>> {
        let id_str = id.0.to_string();
        
        self.connection
            .call(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, page_id, url, title, content_html, content_text, media_files, archived_at, file_size, checksum \
                     FROM content_archives WHERE id = ?1"
                )?;
                
                let result = stmt.query_row([&id_str], row_to_archive);
                
                match result {
                    Ok(archive) => Ok(Some(archive)),
                    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                    Err(e) => Err(e.into()),
                }
            })
            .await
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to get archive: {}", e),
                },
            })
    }

    async fn get_by_page_id(&self, page_id: &Uuid) -> Result<Option<ContentArchive>> {
        let page_id_str = page_id.to_string();
        
        self.connection
            .call(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, page_id, url, title, content_html, content_text, media_files, archived_at, file_size, checksum \
                     FROM content_archives WHERE page_id = ?1 ORDER BY archived_at DESC LIMIT 1"
                )?;
                
                let result = stmt.query_row([&page_id_str], row_to_archive);
                
                match result {
                    Ok(archive) => Ok(Some(archive)),
                    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                    Err(e) => Err(e.into()),
                }
            })
            .await
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to get archive by page ID: {}", e),
                },
            })
    }

    async fn delete(&self, id: &ArchiveId) -> Result<()> {
        let id_str = id.0.to_string();
        
        self.connection
            .call(move |conn| {
                conn.execute(
                    "DELETE FROM content_archives WHERE id = ?1",
                    [&id_str],
                )?;
                Ok(())
            })
            .await
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to delete archive: {}", e),
                },
            })?;
        
        Ok(())
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<ContentArchive>> {
        let query_str = format!("{}*", query.replace('"', "\"\""));
        
        self.connection
            .call(move |conn| {
                let mut stmt = conn.prepare(
                    r#"
                    SELECT a.id, a.page_id, a.url, a.title, a.content_html, a.content_text, 
                           a.media_files, a.archived_at, a.file_size, a.checksum
                    FROM content_archives a
                    JOIN archives_fts fts ON a.rowid = fts.rowid
                    WHERE archives_fts MATCH ?1
                    ORDER BY rank
                    LIMIT ?2
                    "#
                )?;
                
                let rows = stmt.query_map(rusqlite::params![query_str, limit as i64], row_to_archive)?;
                let mut archives = Vec::new();
                for row in rows {
                    if let Ok(archive) = row {
                        archives.push(archive);
                    }
                }
                Ok(archives)
            })
            .await
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to search archives: {}", e),
                },
            })
    }

    async fn get_total_size(&self) -> Result<u64> {
        self.connection
            .call(|conn| {
                let size: i64 = conn.query_row(
                    "SELECT COALESCE(SUM(file_size), 0) FROM content_archives",
                    [],
                    |row| row.get(0),
                )?;
                Ok(size as u64)
            })
            .await
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to get total archive size: {}", e),
                },
            })
    }
}

/// Helper function to map a row to ContentArchive
fn row_to_archive(row: &Row) -> rusqlite::Result<ContentArchive> {
    let id_str: String = row.get(0)?;
    let page_id_str: String = row.get(1)?;
    let url: String = row.get(2)?;
    let title: String = row.get(3)?;
    let content_html: String = row.get(4)?;
    let content_text: String = row.get(5)?;
    let media_files_json: String = row.get(6)?;
    let archived_at_ts: i64 = row.get(7)?;
    let file_size: i64 = row.get(8)?;
    let checksum: Option<String> = row.get(9)?;

    let id = ArchiveId(Uuid::parse_str(&id_str).unwrap_or_else(|_| Uuid::new_v4()));
    let page_id = Uuid::parse_str(&page_id_str).unwrap_or_else(|_| Uuid::new_v4());
    let media_files: Vec<String> = serde_json::from_str(&media_files_json).unwrap_or_default();

    Ok(ContentArchive {
        id,
        page_id,
        url,
        title,
        content_html,
        content_text,
        media_files,
        archived_at: DateTime::from_timestamp(archived_at_ts, 0).unwrap_or_else(Utc::now),
        file_size: file_size as u64,
        checksum,
    })
}

/// Unified search result across all data types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnifiedSearchResult {
    Page(UnifiedPageInfo),
    History(HistoryEntry),
    Archive(ContentArchive),
}

/// Unified search repository for cross-data-source searching
pub struct UnifiedSearchRepository {
    page_repo: SqlitePageRepository,
    history_repo: SqliteHistoryRepository,
    archive_repo: SqliteArchiveRepository,
}

impl UnifiedSearchRepository {
    pub fn new(connection: Arc<Connection>) -> Self {
        Self {
            page_repo: SqlitePageRepository::new(Arc::clone(&connection)),
            history_repo: SqliteHistoryRepository::new(Arc::clone(&connection)),
            archive_repo: SqliteArchiveRepository::new(connection),
        }
    }

    /// Search across all data sources and return unified results
    pub async fn search(&self, query: &str, limit_per_source: usize) -> Result<Vec<UnifiedSearchResult>> {
        let pages = self.page_repo.search_with_limit(query, limit_per_source).await?;
        let history = self.history_repo.search(query, limit_per_source).await?;
        let archives = self.archive_repo.search(query, limit_per_source).await?;

        let mut results: Vec<UnifiedSearchResult> = Vec::new();
        
        for page in pages {
            results.push(UnifiedSearchResult::Page(page));
        }
        
        for entry in history {
            results.push(UnifiedSearchResult::History(entry));
        }
        
        for archive in archives {
            results.push(UnifiedSearchResult::Archive(archive));
        }

        Ok(results)
    }
}
