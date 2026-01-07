//! Repository implementations for data access

use web_page_manager_core::*;
use tokio_rusqlite::Connection;
use std::sync::Arc;
use async_trait::async_trait;

/// Repository trait for unified pages
#[async_trait]
pub trait PageRepository: Send + Sync {
    async fn save(&self, page: &UnifiedPageInfo) -> Result<()>;
    async fn get_by_id(&self, id: &Uuid) -> Result<Option<UnifiedPageInfo>>;
    async fn get_by_url(&self, url: &str) -> Result<Option<UnifiedPageInfo>>;
    async fn get_all(&self) -> Result<Vec<UnifiedPageInfo>>;
    async fn delete(&self, id: &Uuid) -> Result<()>;
    async fn search(&self, query: &str) -> Result<Vec<UnifiedPageInfo>>;
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
                
                conn.execute(
                    r#"
                    INSERT OR REPLACE INTO unified_pages 
                    (id, url, title, favicon_url, content_summary, keywords, category, 
                     source_type, browser_info, created_at, last_accessed, access_count)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
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
                    "SELECT * FROM unified_pages WHERE id = ?1"
                )?;
                
                let result = stmt.query_row([&id_str], |_row| {
                    // TODO: Implement row mapping
                    Ok(None)
                });
                
                match result {
                    Ok(page) => Ok(page),
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
                    "SELECT * FROM unified_pages WHERE url = ?1"
                )?;
                
                let result = stmt.query_row([&url_str], |_row| {
                    // TODO: Implement row mapping
                    Ok(None)
                });
                
                match result {
                    Ok(page) => Ok(page),
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
        // TODO: Implement full retrieval
        Ok(Vec::new())
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
        let query_str = query.to_string();
        
        self.connection
            .call(move |conn| {
                let mut stmt = conn.prepare(
                    r#"
                    SELECT p.* FROM unified_pages p
                    JOIN pages_fts fts ON p.rowid = fts.rowid
                    WHERE pages_fts MATCH ?1
                    ORDER BY rank
                    "#
                )?;
                
                let _rows = stmt.query([&query_str])?;
                
                // TODO: Implement row mapping
                Ok(Vec::new())
            })
            .await
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to search pages: {}", e),
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
                    "SELECT * FROM smart_groups WHERE id = ?1"
                )?;
                
                let result = stmt.query_row([&id_str], |_row| {
                    // TODO: Implement row mapping
                    Ok(None)
                });
                
                match result {
                    Ok(group) => Ok(group),
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
        // TODO: Implement full retrieval
        Ok(Vec::new())
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
                        chrono::Utc::now().timestamp(),
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
}
