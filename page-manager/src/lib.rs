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
//!
//! # Requirements Implemented
//! - 6.1: Display bookmark association marks when tab URL matches existing bookmark
//! - 6.2: Detect tab content changes and offer bookmark info update options
//! - 6.3: Auto-inherit analyzed content summary and tags when adding tab as bookmark

pub mod unified_manager;
pub mod matcher;
pub mod sync;

pub use unified_manager::*;
pub use matcher::*;
pub use sync::*;

// Re-export commonly used types
pub use web_page_manager_core::*;
