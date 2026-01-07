//! Data Access Layer for Web Page Manager
//! 
//! This module provides database operations and data persistence
//! using SQLite with FTS5 for full-text search.

pub mod schema;
pub mod repository;

pub use repository::*;

use web_page_manager_core::*;
use std::path::Path;
use tokio_rusqlite::Connection;
use std::sync::Arc;

/// Database manager for handling SQLite connections
pub struct DatabaseManager {
    connection: Arc<Connection>,
}

impl DatabaseManager {
    /// Create a new database manager with the specified path
    pub async fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let path = db_path.as_ref().to_path_buf();
        
        let connection = Connection::open(path)
            .await
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to open database: {}", e),
                },
            })?;
        
        let manager = Self {
            connection: Arc::new(connection),
        };
        
        // Initialize schema
        manager.initialize_schema().await?;
        
        Ok(manager)
    }

    /// Create an in-memory database (for testing)
    pub async fn in_memory() -> Result<Self> {
        let connection = Connection::open(":memory:")
            .await
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to create in-memory database: {}", e),
                },
            })?;
        
        let manager = Self {
            connection: Arc::new(connection),
        };
        
        manager.initialize_schema().await?;
        
        Ok(manager)
    }

    /// Initialize database schema
    async fn initialize_schema(&self) -> Result<()> {
        self.connection
            .call(|conn| {
                conn.execute_batch(schema::SCHEMA_SQL)?;
                Ok(())
            })
            .await
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to initialize schema: {}", e),
                },
            })?;
        
        Ok(())
    }

    /// Get the connection for repository operations
    pub fn connection(&self) -> Arc<Connection> {
        Arc::clone(&self.connection)
    }
}
