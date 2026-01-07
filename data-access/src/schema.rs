//! Database schema definitions and migrations

/// Current schema version
pub const SCHEMA_VERSION: u32 = 1;

/// SQL schema for the Web Page Manager database
pub const SCHEMA_SQL: &str = r#"
-- Schema version tracking
CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    applied_at INTEGER NOT NULL,
    description TEXT
);

-- Unified pages table
CREATE TABLE IF NOT EXISTS unified_pages (
    id TEXT PRIMARY KEY,
    url TEXT NOT NULL,
    title TEXT NOT NULL,
    favicon_url TEXT,
    content_summary TEXT, -- JSON
    keywords TEXT, -- JSON array
    category TEXT,
    source_type TEXT NOT NULL, -- JSON
    browser_info TEXT, -- JSON
    tab_info TEXT, -- JSON
    bookmark_info TEXT, -- JSON
    created_at INTEGER NOT NULL,
    last_accessed INTEGER NOT NULL,
    access_count INTEGER DEFAULT 0
);

-- Smart groups table
CREATE TABLE IF NOT EXISTS smart_groups (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    group_type TEXT NOT NULL, -- JSON
    created_at INTEGER NOT NULL,
    auto_generated BOOLEAN DEFAULT FALSE,
    similarity_threshold REAL
);

-- Page group relations table
CREATE TABLE IF NOT EXISTS page_group_relations (
    page_id TEXT NOT NULL,
    group_id TEXT NOT NULL,
    added_at INTEGER NOT NULL,
    confidence_score REAL,
    PRIMARY KEY (page_id, group_id),
    FOREIGN KEY (page_id) REFERENCES unified_pages(id) ON DELETE CASCADE,
    FOREIGN KEY (group_id) REFERENCES smart_groups(id) ON DELETE CASCADE
);

-- Tab history table
CREATE TABLE IF NOT EXISTS tab_history (
    id TEXT PRIMARY KEY,
    page_id TEXT,
    url TEXT NOT NULL,
    title TEXT NOT NULL,
    favicon_url TEXT,
    browser_type TEXT NOT NULL,
    tab_id TEXT,
    closed_at INTEGER NOT NULL,
    session_info TEXT, -- JSON
    content_summary TEXT, -- JSON
    FOREIGN KEY (page_id) REFERENCES unified_pages(id) ON DELETE SET NULL
);

-- Content archives table
CREATE TABLE IF NOT EXISTS content_archives (
    id TEXT PRIMARY KEY,
    page_id TEXT NOT NULL,
    url TEXT NOT NULL,
    title TEXT NOT NULL,
    content_html TEXT NOT NULL,
    content_text TEXT NOT NULL,
    media_files TEXT, -- JSON array
    archived_at INTEGER NOT NULL,
    file_size INTEGER,
    checksum TEXT,
    FOREIGN KEY (page_id) REFERENCES unified_pages(id) ON DELETE CASCADE
);

-- Full-text search index for pages
CREATE VIRTUAL TABLE IF NOT EXISTS pages_fts USING fts5(
    title, 
    content_summary, 
    keywords,
    url,
    content='unified_pages',
    content_rowid='rowid',
    tokenize='porter unicode61'
);

-- Full-text search index for archives
CREATE VIRTUAL TABLE IF NOT EXISTS archives_fts USING fts5(
    title,
    content_text,
    url,
    content='content_archives',
    content_rowid='rowid',
    tokenize='porter unicode61'
);

-- Full-text search index for history
CREATE VIRTUAL TABLE IF NOT EXISTS history_fts USING fts5(
    title,
    url,
    content='tab_history',
    content_rowid='rowid',
    tokenize='porter unicode61'
);

-- Triggers to maintain pages FTS index
CREATE TRIGGER IF NOT EXISTS pages_fts_insert AFTER INSERT ON unified_pages BEGIN
    INSERT INTO pages_fts(rowid, title, content_summary, keywords, url) 
    VALUES (new.rowid, new.title, new.content_summary, new.keywords, new.url);
END;

CREATE TRIGGER IF NOT EXISTS pages_fts_delete AFTER DELETE ON unified_pages BEGIN
    INSERT INTO pages_fts(pages_fts, rowid, title, content_summary, keywords, url) 
    VALUES ('delete', old.rowid, old.title, old.content_summary, old.keywords, old.url);
END;

CREATE TRIGGER IF NOT EXISTS pages_fts_update AFTER UPDATE ON unified_pages BEGIN
    INSERT INTO pages_fts(pages_fts, rowid, title, content_summary, keywords, url) 
    VALUES ('delete', old.rowid, old.title, old.content_summary, old.keywords, old.url);
    INSERT INTO pages_fts(rowid, title, content_summary, keywords, url) 
    VALUES (new.rowid, new.title, new.content_summary, new.keywords, new.url);
END;

-- Triggers to maintain archives FTS index
CREATE TRIGGER IF NOT EXISTS archives_fts_insert AFTER INSERT ON content_archives BEGIN
    INSERT INTO archives_fts(rowid, title, content_text, url) 
    VALUES (new.rowid, new.title, new.content_text, new.url);
END;

CREATE TRIGGER IF NOT EXISTS archives_fts_delete AFTER DELETE ON content_archives BEGIN
    INSERT INTO archives_fts(archives_fts, rowid, title, content_text, url) 
    VALUES ('delete', old.rowid, old.title, old.content_text, old.url);
END;

CREATE TRIGGER IF NOT EXISTS archives_fts_update AFTER UPDATE ON content_archives BEGIN
    INSERT INTO archives_fts(archives_fts, rowid, title, content_text, url) 
    VALUES ('delete', old.rowid, old.title, old.content_text, old.url);
    INSERT INTO archives_fts(rowid, title, content_text, url) 
    VALUES (new.rowid, new.title, new.content_text, new.url);
END;

-- Triggers to maintain history FTS index
CREATE TRIGGER IF NOT EXISTS history_fts_insert AFTER INSERT ON tab_history BEGIN
    INSERT INTO history_fts(rowid, title, url) 
    VALUES (new.rowid, new.title, new.url);
END;

CREATE TRIGGER IF NOT EXISTS history_fts_delete AFTER DELETE ON tab_history BEGIN
    INSERT INTO history_fts(history_fts, rowid, title, url) 
    VALUES ('delete', old.rowid, old.title, old.url);
END;

CREATE TRIGGER IF NOT EXISTS history_fts_update AFTER UPDATE ON tab_history BEGIN
    INSERT INTO history_fts(history_fts, rowid, title, url) 
    VALUES ('delete', old.rowid, old.title, old.url);
    INSERT INTO history_fts(rowid, title, url) 
    VALUES (new.rowid, new.title, new.url);
END;

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_unified_pages_url ON unified_pages(url);
CREATE INDEX IF NOT EXISTS idx_unified_pages_created_at ON unified_pages(created_at);
CREATE INDEX IF NOT EXISTS idx_unified_pages_last_accessed ON unified_pages(last_accessed);
CREATE INDEX IF NOT EXISTS idx_unified_pages_category ON unified_pages(category);
CREATE INDEX IF NOT EXISTS idx_tab_history_closed_at ON tab_history(closed_at);
CREATE INDEX IF NOT EXISTS idx_tab_history_browser_type ON tab_history(browser_type);
CREATE INDEX IF NOT EXISTS idx_tab_history_url ON tab_history(url);
CREATE INDEX IF NOT EXISTS idx_content_archives_archived_at ON content_archives(archived_at);
CREATE INDEX IF NOT EXISTS idx_content_archives_url ON content_archives(url);
CREATE INDEX IF NOT EXISTS idx_content_archives_page_id ON content_archives(page_id);
CREATE INDEX IF NOT EXISTS idx_page_group_relations_group_id ON page_group_relations(group_id);
"#;

/// Migration definitions
pub struct Migration {
    pub version: u32,
    pub description: &'static str,
    pub sql: &'static str,
}

/// List of all migrations
pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        description: "Initial schema",
        sql: SCHEMA_SQL,
    },
];

/// Get migration by version
pub fn get_migration(version: u32) -> Option<&'static Migration> {
    MIGRATIONS.iter().find(|m| m.version == version)
}
