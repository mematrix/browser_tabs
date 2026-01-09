//! Page Unified Manager for Web Page Manager
//!
//! This module provides unified management of tabs and bookmarks,
//! handling data merging, association matching, and synchronization.
//!
//! # Features
//! - Unified page information management system
//! - Tab and bookmark association matching
//! - Data synchronization and update mechanism
//! - Cross-reference recommendations
//! - Unified search across all data sources
//! - Tab history management with rich information
//!
//! # Requirements Implemented
//! - 6.1: Display bookmark association marks when tab URL matches existing bookmark
//! - 6.2: Detect tab content changes and offer bookmark info update options
//! - 6.3: Auto-inherit analyzed content summary and tags when adding tab as bookmark
//! - 6.5: Unified search across tabs and bookmarks with comprehensive results
//! - 7.1: Auto-save closed tab complete information to history
//! - 7.2: Preserve page title, URL, close time, and analyzed content summary
//! - 7.3: Display richer information than browser history including content preview and tags

pub mod unified_manager;
pub mod matcher;
pub mod sync;
pub mod search;
pub mod history;

pub use unified_manager::*;
pub use matcher::*;
pub use sync::*;
pub use search::*;
pub use history::*;

// Re-export commonly used types
pub use web_page_manager_core::*;
