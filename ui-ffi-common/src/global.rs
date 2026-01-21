//! Global state management for the UI FFI layer.

use std::sync::{LazyLock, OnceLock};

use data_access::DatabaseManager;
use page_manager::UnifiedSearchManager;

static DATABASE_MANAGER: OnceLock<DatabaseManager> = OnceLock::new();

/// Initializes the required global instance.
pub async fn init_once() {
    // todo: generate or configure the database path as needed. handle error.
    let db = DatabaseManager::new("local.db").await.unwrap();
    let _ = DATABASE_MANAGER.set(db);
}

pub fn database_manager() -> &'static DatabaseManager {
    DATABASE_MANAGER
        .get()
        .expect("DatabaseManager is not initialized. Call init_once() first.")
}

static SEARCH_MANAGER: LazyLock<UnifiedSearchManager> =
    LazyLock::new(|| UnifiedSearchManager::new(database_manager()));

pub fn search_manager() -> &'static UnifiedSearchManager {
    &SEARCH_MANAGER
}
