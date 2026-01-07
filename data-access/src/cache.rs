//! Data caching layer for Web Page Manager
//!
//! Implements LRU caching for frequently accessed data with TTL support.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use web_page_manager_core::*;

/// Cache entry with value and metadata
struct CacheEntry<V> {
    value: V,
    inserted_at: Instant,
    last_accessed: Instant,
    access_count: u64,
}

impl<V: Clone> CacheEntry<V> {
    fn new(value: V) -> Self {
        let now = Instant::now();
        Self {
            value,
            inserted_at: now,
            last_accessed: now,
            access_count: 1,
        }
    }

    fn is_expired(&self, ttl: Duration) -> bool {
        self.inserted_at.elapsed() > ttl
    }

    fn touch(&mut self) {
        self.last_accessed = Instant::now();
        self.access_count += 1;
    }
}

/// LRU cache with TTL support
pub struct LruCache<K, V> {
    entries: HashMap<K, CacheEntry<V>>,
    max_size: usize,
    ttl: Duration,
    order: Vec<K>,
}

impl<K: Eq + Hash + Clone, V: Clone> LruCache<K, V> {
    pub fn new(max_size: usize, ttl: Duration) -> Self {
        Self {
            entries: HashMap::with_capacity(max_size),
            max_size,
            ttl,
            order: Vec::with_capacity(max_size),
        }
    }

    pub fn get(&mut self, key: &K) -> Option<V> {
        if let Some(entry) = self.entries.get_mut(key) {
            if entry.is_expired(self.ttl) {
                self.remove(key);
                return None;
            }
            entry.touch();
            // Move to end of order (most recently used)
            if let Some(pos) = self.order.iter().position(|k| k == key) {
                self.order.remove(pos);
                self.order.push(key.clone());
            }
            Some(entry.value.clone())
        } else {
            None
        }
    }

    pub fn insert(&mut self, key: K, value: V) {
        // Remove if already exists
        self.remove(&key);

        // Evict oldest if at capacity
        while self.entries.len() >= self.max_size && !self.order.is_empty() {
            let oldest_key = self.order.remove(0);
            self.entries.remove(&oldest_key);
        }

        self.entries.insert(key.clone(), CacheEntry::new(value));
        self.order.push(key);
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        if let Some(pos) = self.order.iter().position(|k| k == key) {
            self.order.remove(pos);
        }
        self.entries.remove(key).map(|e| e.value)
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Remove expired entries
    pub fn cleanup_expired(&mut self) {
        let expired_keys: Vec<K> = self
            .entries
            .iter()
            .filter(|(_, entry)| entry.is_expired(self.ttl))
            .map(|(k, _)| k.clone())
            .collect();

        for key in expired_keys {
            self.remove(&key);
        }
    }
}

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum number of pages in cache
    pub max_pages: usize,
    /// Maximum number of summaries in cache
    pub max_summaries: usize,
    /// Maximum number of groups in cache
    pub max_groups: usize,
    /// TTL for page cache entries
    pub page_ttl: Duration,
    /// TTL for summary cache entries
    pub summary_ttl: Duration,
    /// TTL for group cache entries
    pub group_ttl: Duration,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_pages: 1000,
            max_summaries: 500,
            max_groups: 100,
            page_ttl: Duration::from_secs(3600),      // 1 hour
            summary_ttl: Duration::from_secs(1800),   // 30 minutes
            group_ttl: Duration::from_secs(1800),     // 30 minutes
        }
    }
}

/// Thread-safe data cache manager
pub struct DataCache {
    pages: Arc<RwLock<LruCache<Uuid, UnifiedPageInfo>>>,
    pages_by_url: Arc<RwLock<LruCache<String, Uuid>>>,
    summaries: Arc<RwLock<LruCache<Uuid, ContentSummary>>>,
    groups: Arc<RwLock<LruCache<Uuid, SmartGroup>>>,
    config: CacheConfig,
}

impl DataCache {
    pub fn new(config: CacheConfig) -> Self {
        Self {
            pages: Arc::new(RwLock::new(LruCache::new(config.max_pages, config.page_ttl))),
            pages_by_url: Arc::new(RwLock::new(LruCache::new(config.max_pages, config.page_ttl))),
            summaries: Arc::new(RwLock::new(LruCache::new(config.max_summaries, config.summary_ttl))),
            groups: Arc::new(RwLock::new(LruCache::new(config.max_groups, config.group_ttl))),
            config,
        }
    }

    /// Get a page by ID from cache
    pub async fn get_page(&self, id: &Uuid) -> Option<UnifiedPageInfo> {
        let mut cache = self.pages.write().await;
        cache.get(id)
    }

    /// Get a page ID by URL from cache
    pub async fn get_page_id_by_url(&self, url: &str) -> Option<Uuid> {
        let mut cache = self.pages_by_url.write().await;
        cache.get(&url.to_string())
    }

    /// Cache a page
    pub async fn cache_page(&self, page: &UnifiedPageInfo) {
        let mut pages_cache = self.pages.write().await;
        let mut url_cache = self.pages_by_url.write().await;
        
        pages_cache.insert(page.id, page.clone());
        url_cache.insert(page.url.clone(), page.id);
    }

    /// Invalidate a page from cache
    pub async fn invalidate_page(&self, id: &Uuid) {
        let mut pages_cache = self.pages.write().await;
        if let Some(page) = pages_cache.remove(id) {
            let mut url_cache = self.pages_by_url.write().await;
            url_cache.remove(&page.url);
        }
    }

    /// Get a content summary from cache
    pub async fn get_summary(&self, page_id: &Uuid) -> Option<ContentSummary> {
        let mut cache = self.summaries.write().await;
        cache.get(page_id)
    }

    /// Cache a content summary
    pub async fn cache_summary(&self, page_id: Uuid, summary: &ContentSummary) {
        let mut cache = self.summaries.write().await;
        cache.insert(page_id, summary.clone());
    }

    /// Invalidate a summary from cache
    pub async fn invalidate_summary(&self, page_id: &Uuid) {
        let mut cache = self.summaries.write().await;
        cache.remove(page_id);
    }

    /// Get a group from cache
    pub async fn get_group(&self, id: &Uuid) -> Option<SmartGroup> {
        let mut cache = self.groups.write().await;
        cache.get(id)
    }

    /// Cache a group
    pub async fn cache_group(&self, group: &SmartGroup) {
        let mut cache = self.groups.write().await;
        cache.insert(group.id, group.clone());
    }

    /// Invalidate a group from cache
    pub async fn invalidate_group(&self, id: &Uuid) {
        let mut cache = self.groups.write().await;
        cache.remove(id);
    }

    /// Clear all caches
    pub async fn clear_all(&self) {
        let mut pages = self.pages.write().await;
        let mut urls = self.pages_by_url.write().await;
        let mut summaries = self.summaries.write().await;
        let mut groups = self.groups.write().await;
        
        pages.clear();
        urls.clear();
        summaries.clear();
        groups.clear();
    }

    /// Cleanup expired entries from all caches
    pub async fn cleanup_expired(&self) {
        let mut pages = self.pages.write().await;
        let mut urls = self.pages_by_url.write().await;
        let mut summaries = self.summaries.write().await;
        let mut groups = self.groups.write().await;
        
        pages.cleanup_expired();
        urls.cleanup_expired();
        summaries.cleanup_expired();
        groups.cleanup_expired();
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        let pages = self.pages.read().await;
        let urls = self.pages_by_url.read().await;
        let summaries = self.summaries.read().await;
        let groups = self.groups.read().await;
        
        CacheStats {
            pages_count: pages.len(),
            pages_max: self.config.max_pages,
            urls_count: urls.len(),
            summaries_count: summaries.len(),
            summaries_max: self.config.max_summaries,
            groups_count: groups.len(),
            groups_max: self.config.max_groups,
        }
    }
}

impl Default for DataCache {
    fn default() -> Self {
        Self::new(CacheConfig::default())
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub pages_count: usize,
    pub pages_max: usize,
    pub urls_count: usize,
    pub summaries_count: usize,
    pub summaries_max: usize,
    pub groups_count: usize,
    pub groups_max: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lru_cache_basic() {
        let mut cache: LruCache<String, i32> = LruCache::new(3, Duration::from_secs(60));
        
        cache.insert("a".to_string(), 1);
        cache.insert("b".to_string(), 2);
        cache.insert("c".to_string(), 3);
        
        assert_eq!(cache.get(&"a".to_string()), Some(1));
        assert_eq!(cache.get(&"b".to_string()), Some(2));
        assert_eq!(cache.get(&"c".to_string()), Some(3));
        assert_eq!(cache.len(), 3);
    }

    #[test]
    fn test_lru_cache_eviction() {
        let mut cache: LruCache<String, i32> = LruCache::new(2, Duration::from_secs(60));
        
        cache.insert("a".to_string(), 1);
        cache.insert("b".to_string(), 2);
        cache.insert("c".to_string(), 3); // Should evict "a"
        
        assert_eq!(cache.get(&"a".to_string()), None);
        assert_eq!(cache.get(&"b".to_string()), Some(2));
        assert_eq!(cache.get(&"c".to_string()), Some(3));
    }

    #[test]
    fn test_lru_cache_access_order() {
        let mut cache: LruCache<String, i32> = LruCache::new(2, Duration::from_secs(60));
        
        cache.insert("a".to_string(), 1);
        cache.insert("b".to_string(), 2);
        
        // Access "a" to make it most recently used
        cache.get(&"a".to_string());
        
        // Insert "c", should evict "b" (least recently used)
        cache.insert("c".to_string(), 3);
        
        assert_eq!(cache.get(&"a".to_string()), Some(1));
        assert_eq!(cache.get(&"b".to_string()), None);
        assert_eq!(cache.get(&"c".to_string()), Some(3));
    }

    #[tokio::test]
    async fn test_data_cache_pages() {
        let cache = DataCache::new(CacheConfig::default());
        
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
        
        cache.cache_page(&page).await;
        
        let cached = cache.get_page(&page.id).await;
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().url, "https://example.com");
        
        let cached_by_url = cache.get_page_id_by_url("https://example.com").await;
        assert!(cached_by_url.is_some());
        assert_eq!(cached_by_url.unwrap(), page.id);
    }
}
