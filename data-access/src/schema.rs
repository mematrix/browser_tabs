//! Database schema definitions

/// SQL schema for the Web Page Manager database
pub const SCHEMA_SQL: &str = r#"
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
    created_at INTEGER NOT NULL,
    last_accessed INTEGER NOT NULL,
    access_count INTEGER DEFAULT 0,
    UNIQUE(url, source_type)
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
    page_id TEXT NOT NULL,
    browser_type TEXT NOT NULL,
    tab_id TEXT,
    closed_at INTEGER NOT NULL,
    session_info TEXT, -- JSON
    FOREIGN KEY (page_id) REFERENCES unified_pages(id) ON DELETE CASCADE
);

-- Content archives table
CREATE TABLE IF NOT EXISTS content_archives (
    id TEXT PRIMARY KEY,
    page_id TEXT NOT NULL,
    content_html TEXT NOT NULL,
    content_text TEXT NOT NULL,
    media_files TEXT, -- JSON array
    archived_at INTEGER NOT NULL,
    file_size INTEGER,
    FOREIGN KEY (page_id) REFERENCES unified_pages(id) ON DELETE CASCADE
);

-- Full-text search index
CREATE VIRTUAL TABLE IF NOT EXISTS pages_fts USING fts5(
    title, 
    content_summary, 
    keywords,
    content='unified_pages',
    content_rowid='rowid'
);

-- Triggers to maintain FTS index
CREATE TRIGGER IF NOT EXISTS pages_fts_insert AFTER INSERT ON unified_pages BEGIN
    INSERT INTO pages_fts(rowid, title, content_summary, keywords) 
    VALUES (new.rowid, new.title, new.content_summary, new.keywords);
END;

CREATE TRIGGER IF NOT EXISTS pages_fts_delete AFTER DELETE ON unified_pages BEGIN
    INSERT INTO pages_fts(pages_fts, rowid, title, content_summary, keywords) 
    VALUES ('delete', old.rowid, old.title, old.content_summary, old.keywords);
END;

CREATE TRIGGER IF NOT EXISTS pages_fts_update AFTER UPDATE ON unified_pages BEGIN
    INSERT INTO pages_fts(pages_fts, rowid, title, content_summary, keywords) 
    VALUES ('delete', old.rowid, old.title, old.content_summary, old.keywords);
    INSERT INTO pages_fts(rowid, title, content_summary, keywords) 
    VALUES (new.rowid, new.title, new.content_summary, new.keywords);
END;

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_unified_pages_url ON unified_pages(url);
CREATE INDEX IF NOT EXISTS idx_unified_pages_created_at ON unified_pages(created_at);
CREATE INDEX IF NOT EXISTS idx_unified_pages_last_accessed ON unified_pages(last_accessed);
CREATE INDEX IF NOT EXISTS idx_tab_history_closed_at ON tab_history(closed_at);
CREATE INDEX IF NOT EXISTS idx_content_archives_archived_at ON content_archives(archived_at);
"#;
