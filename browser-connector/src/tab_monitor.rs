//! Tab state monitoring module
//!
//! This module provides functionality to monitor tab state changes across
//! multiple browsers, including tab creation, closure, navigation, and updates.

use web_page_manager_core::{BrowserType, TabId, TabInfo, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use chrono::{DateTime, Duration};

/// Events that can occur on tabs
#[derive(Debug, Clone)]
pub enum TabEvent {
    /// A new tab was created
    Created {
        tab: TabInfo,
        timestamp: DateTime<chrono::Utc>,
    },
    /// A tab was closed
    Closed {
        tab_id: TabId,
        browser_type: BrowserType,
        timestamp: DateTime<chrono::Utc>,
        /// The tab info at the time of closure (if available)
        last_known_info: Option<TabInfo>,
    },
    /// A tab's URL changed (navigation)
    Navigated {
        tab_id: TabId,
        browser_type: BrowserType,
        old_url: String,
        new_url: String,
        timestamp: DateTime<chrono::Utc>,
    },
    /// A tab's title changed
    TitleChanged {
        tab_id: TabId,
        browser_type: BrowserType,
        old_title: String,
        new_title: String,
        timestamp: DateTime<chrono::Utc>,
    },
    /// A tab was activated (brought to focus)
    Activated {
        tab_id: TabId,
        browser_type: BrowserType,
        timestamp: DateTime<chrono::Utc>,
    },
    /// A tab's loading state changed
    LoadingStateChanged {
        tab_id: TabId,
        browser_type: BrowserType,
        is_loading: bool,
        timestamp: DateTime<chrono::Utc>,
    },
}

/// Tab state snapshot for comparison
#[derive(Debug, Clone)]
pub struct TabSnapshot {
    pub tab: TabInfo,
    pub captured_at: DateTime<chrono::Utc>,
}

/// Configuration for the tab monitor
#[derive(Debug, Clone)]
pub struct TabMonitorConfig {
    /// Interval between polling for tab changes (in milliseconds)
    pub poll_interval_ms: u64,
    /// Maximum number of events to keep in history
    pub max_event_history: usize,
    /// Whether to track navigation events
    pub track_navigation: bool,
    /// Whether to track title changes
    pub track_title_changes: bool,
    /// Whether to emit events for browser internal pages
    pub include_internal_pages: bool,
}

impl Default for TabMonitorConfig {
    fn default() -> Self {
        Self {
            poll_interval_ms: 1000,
            max_event_history: 1000,
            track_navigation: true,
            track_title_changes: true,
            include_internal_pages: false,
        }
    }
}

/// Tab state monitor that tracks changes across browsers
pub struct TabMonitor {
    /// Current known state of all tabs
    tab_states: Arc<RwLock<HashMap<(BrowserType, TabId), TabSnapshot>>>,
    /// Event history
    event_history: Arc<RwLock<Vec<TabEvent>>>,
    /// Configuration
    config: TabMonitorConfig,
    /// Event sender for broadcasting events
    event_sender: Option<mpsc::Sender<TabEvent>>,
    /// Whether the monitor is running (reserved for future background polling)
    #[allow(dead_code)]
    is_running: Arc<RwLock<bool>>,
}

impl TabMonitor {
    /// Create a new tab monitor with default configuration
    pub fn new() -> Self {
        Self {
            tab_states: Arc::new(RwLock::new(HashMap::new())),
            event_history: Arc::new(RwLock::new(Vec::new())),
            config: TabMonitorConfig::default(),
            event_sender: None,
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Create a new tab monitor with custom configuration
    pub fn with_config(config: TabMonitorConfig) -> Self {
        Self {
            tab_states: Arc::new(RwLock::new(HashMap::new())),
            event_history: Arc::new(RwLock::new(Vec::new())),
            config,
            event_sender: None,
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Subscribe to tab events
    /// 
    /// Returns a receiver that will receive all tab events
    pub fn subscribe(&mut self) -> mpsc::Receiver<TabEvent> {
        let (sender, receiver) = mpsc::channel(100);
        self.event_sender = Some(sender);
        receiver
    }

    /// Update the monitor with current tabs from all browsers
    /// 
    /// This method compares the current tabs with the previous state
    /// and generates appropriate events for any changes detected.
    pub async fn update_tabs(&self, browser_tabs: HashMap<BrowserType, Vec<TabInfo>>) -> Vec<TabEvent> {
        let mut events = Vec::new();
        let now = Utc::now();
        
        let mut current_states = self.tab_states.write().await;
        
        // Track which tabs we've seen in this update
        let mut seen_tabs: HashMap<(BrowserType, TabId), bool> = HashMap::new();
        
        // Process each browser's tabs
        for (browser_type, tabs) in browser_tabs {
            for tab in tabs {
                // Skip internal pages if configured
                if !self.config.include_internal_pages && self.is_internal_page(&tab.url) {
                    continue;
                }
                
                let key = (browser_type, tab.id.clone());
                seen_tabs.insert(key.clone(), true);
                
                if let Some(previous) = current_states.get(&key) {
                    // Tab exists - check for changes
                    
                    // Check for navigation
                    if self.config.track_navigation && previous.tab.url != tab.url {
                        let event = TabEvent::Navigated {
                            tab_id: tab.id.clone(),
                            browser_type,
                            old_url: previous.tab.url.clone(),
                            new_url: tab.url.clone(),
                            timestamp: now,
                        };
                        events.push(event);
                    }
                    
                    // Check for title change
                    if self.config.track_title_changes && previous.tab.title != tab.title {
                        let event = TabEvent::TitleChanged {
                            tab_id: tab.id.clone(),
                            browser_type,
                            old_title: previous.tab.title.clone(),
                            new_title: tab.title.clone(),
                            timestamp: now,
                        };
                        events.push(event);
                    }
                } else {
                    // New tab
                    let event = TabEvent::Created {
                        tab: tab.clone(),
                        timestamp: now,
                    };
                    events.push(event);
                }
                
                // Update state
                current_states.insert(key, TabSnapshot {
                    tab,
                    captured_at: now,
                });
            }
        }
        
        // Find closed tabs (tabs that were in previous state but not in current)
        let closed_keys: Vec<_> = current_states.keys()
            .filter(|k| !seen_tabs.contains_key(*k))
            .cloned()
            .collect();
        
        for key in closed_keys {
            if let Some(snapshot) = current_states.remove(&key) {
                let event = TabEvent::Closed {
                    tab_id: key.1,
                    browser_type: key.0,
                    timestamp: now,
                    last_known_info: Some(snapshot.tab),
                };
                events.push(event);
            }
        }
        
        drop(current_states);
        
        // Store events in history
        self.store_events(&events).await;
        
        // Broadcast events
        self.broadcast_events(&events).await;
        
        events
    }

    /// Check if a URL is a browser internal page
    fn is_internal_page(&self, url: &str) -> bool {
        let lower_url = url.to_lowercase();
        lower_url.starts_with("chrome://")
            || lower_url.starts_with("edge://")
            || lower_url.starts_with("about:")
            || lower_url.starts_with("chrome-extension://")
            || lower_url.starts_with("moz-extension://")
    }

    /// Store events in history
    async fn store_events(&self, events: &[TabEvent]) {
        let mut history = self.event_history.write().await;
        
        for event in events {
            history.push(event.clone());
        }
        
        // Trim history if needed
        while history.len() > self.config.max_event_history {
            history.remove(0);
        }
    }

    /// Broadcast events to subscribers
    async fn broadcast_events(&self, events: &[TabEvent]) {
        if let Some(sender) = &self.event_sender {
            for event in events {
                // Ignore send errors (receiver might be dropped)
                let _ = sender.send(event.clone()).await;
            }
        }
    }

    /// Get the current state of all monitored tabs
    pub async fn get_current_tabs(&self) -> Vec<TabInfo> {
        let states = self.tab_states.read().await;
        states.values().map(|s| s.tab.clone()).collect()
    }

    /// Get tabs for a specific browser
    pub async fn get_tabs_for_browser(&self, browser_type: BrowserType) -> Vec<TabInfo> {
        let states = self.tab_states.read().await;
        states.iter()
            .filter(|((bt, _), _)| *bt == browser_type)
            .map(|(_, s)| s.tab.clone())
            .collect()
    }

    /// Get a specific tab by ID
    pub async fn get_tab(&self, browser_type: BrowserType, tab_id: &TabId) -> Option<TabInfo> {
        let states = self.tab_states.read().await;
        states.get(&(browser_type, tab_id.clone())).map(|s| s.tab.clone())
    }

    /// Get recent events
    pub async fn get_recent_events(&self, count: usize) -> Vec<TabEvent> {
        let history = self.event_history.read().await;
        history.iter()
            .rev()
            .take(count)
            .cloned()
            .collect()
    }

    /// Get events within a time range
    pub async fn get_events_in_range(
        &self,
        from: DateTime<chrono::Utc>,
        to: DateTime<chrono::Utc>,
    ) -> Vec<TabEvent> {
        let history = self.event_history.read().await;
        history.iter()
            .filter(|e| {
                let timestamp = match e {
                    TabEvent::Created { timestamp, .. } => timestamp,
                    TabEvent::Closed { timestamp, .. } => timestamp,
                    TabEvent::Navigated { timestamp, .. } => timestamp,
                    TabEvent::TitleChanged { timestamp, .. } => timestamp,
                    TabEvent::Activated { timestamp, .. } => timestamp,
                    TabEvent::LoadingStateChanged { timestamp, .. } => timestamp,
                };
                *timestamp >= from && *timestamp <= to
            })
            .cloned()
            .collect()
    }

    /// Get closed tabs from recent history
    pub async fn get_recently_closed_tabs(&self, within_minutes: i64) -> Vec<TabInfo> {
        let cutoff = Utc::now() - Duration::minutes(within_minutes);
        let history = self.event_history.read().await;
        
        history.iter()
            .filter_map(|e| {
                if let TabEvent::Closed { timestamp, last_known_info, .. } = e {
                    if *timestamp >= cutoff {
                        return last_known_info.clone();
                    }
                }
                None
            })
            .collect()
    }

    /// Clear all monitored state
    pub async fn clear(&self) {
        let mut states = self.tab_states.write().await;
        states.clear();
        
        let mut history = self.event_history.write().await;
        history.clear();
    }

    /// Get statistics about monitored tabs
    pub async fn get_stats(&self) -> TabMonitorStats {
        let states = self.tab_states.read().await;
        let history = self.event_history.read().await;
        
        let mut tabs_by_browser: HashMap<BrowserType, usize> = HashMap::new();
        for ((browser_type, _), _) in states.iter() {
            *tabs_by_browser.entry(*browser_type).or_insert(0) += 1;
        }
        
        let mut events_by_type: HashMap<String, usize> = HashMap::new();
        for event in history.iter() {
            let event_type = match event {
                TabEvent::Created { .. } => "created",
                TabEvent::Closed { .. } => "closed",
                TabEvent::Navigated { .. } => "navigated",
                TabEvent::TitleChanged { .. } => "title_changed",
                TabEvent::Activated { .. } => "activated",
                TabEvent::LoadingStateChanged { .. } => "loading_state_changed",
            };
            *events_by_type.entry(event_type.to_string()).or_insert(0) += 1;
        }
        
        TabMonitorStats {
            total_tabs: states.len(),
            tabs_by_browser,
            total_events: history.len(),
            events_by_type,
        }
    }
}

impl Default for TabMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about monitored tabs
#[derive(Debug, Clone)]
pub struct TabMonitorStats {
    /// Total number of currently monitored tabs
    pub total_tabs: usize,
    /// Number of tabs per browser
    pub tabs_by_browser: HashMap<BrowserType, usize>,
    /// Total number of events in history
    pub total_events: usize,
    /// Number of events by type
    pub events_by_type: HashMap<String, usize>,
}



#[cfg(test)]
mod tests {
    use super::*;
    use web_page_manager_core::TabId;

    fn create_test_tab(id: &str, url: &str, title: &str, browser_type: BrowserType) -> TabInfo {
        TabInfo {
            id: TabId(id.to_string()),
            url: url.to_string(),
            title: title.to_string(),
            favicon_url: None,
            browser_type,
            is_private: false,
            created_at: Utc::now(),
            last_accessed: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_monitor_creation() {
        let monitor = TabMonitor::new();
        let tabs = monitor.get_current_tabs().await;
        assert!(tabs.is_empty());
    }

    #[tokio::test]
    async fn test_detect_new_tabs() {
        let monitor = TabMonitor::new();
        
        let mut browser_tabs = HashMap::new();
        browser_tabs.insert(BrowserType::Chrome, vec![
            create_test_tab("tab1", "https://example.com", "Example", BrowserType::Chrome),
        ]);
        
        let events = monitor.update_tabs(browser_tabs).await;
        
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], TabEvent::Created { .. }));
    }

    #[tokio::test]
    async fn test_detect_closed_tabs() {
        let monitor = TabMonitor::new();
        
        // First update with a tab
        let mut browser_tabs = HashMap::new();
        browser_tabs.insert(BrowserType::Chrome, vec![
            create_test_tab("tab1", "https://example.com", "Example", BrowserType::Chrome),
        ]);
        monitor.update_tabs(browser_tabs).await;
        
        // Second update without the tab
        let browser_tabs = HashMap::new();
        let events = monitor.update_tabs(browser_tabs).await;
        
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], TabEvent::Closed { .. }));
    }

    #[tokio::test]
    async fn test_detect_navigation() {
        let monitor = TabMonitor::new();
        
        // First update
        let mut browser_tabs = HashMap::new();
        browser_tabs.insert(BrowserType::Chrome, vec![
            create_test_tab("tab1", "https://example.com", "Example", BrowserType::Chrome),
        ]);
        monitor.update_tabs(browser_tabs).await;
        
        // Second update with different URL
        let mut browser_tabs = HashMap::new();
        browser_tabs.insert(BrowserType::Chrome, vec![
            create_test_tab("tab1", "https://example.org", "Example", BrowserType::Chrome),
        ]);
        let events = monitor.update_tabs(browser_tabs).await;
        
        assert_eq!(events.len(), 1);
        if let TabEvent::Navigated { old_url, new_url, .. } = &events[0] {
            assert_eq!(old_url, "https://example.com");
            assert_eq!(new_url, "https://example.org");
        } else {
            panic!("Expected Navigated event");
        }
    }

    #[tokio::test]
    async fn test_detect_title_change() {
        let monitor = TabMonitor::new();
        
        // First update
        let mut browser_tabs = HashMap::new();
        browser_tabs.insert(BrowserType::Chrome, vec![
            create_test_tab("tab1", "https://example.com", "Old Title", BrowserType::Chrome),
        ]);
        monitor.update_tabs(browser_tabs).await;
        
        // Second update with different title
        let mut browser_tabs = HashMap::new();
        browser_tabs.insert(BrowserType::Chrome, vec![
            create_test_tab("tab1", "https://example.com", "New Title", BrowserType::Chrome),
        ]);
        let events = monitor.update_tabs(browser_tabs).await;
        
        assert_eq!(events.len(), 1);
        if let TabEvent::TitleChanged { old_title, new_title, .. } = &events[0] {
            assert_eq!(old_title, "Old Title");
            assert_eq!(new_title, "New Title");
        } else {
            panic!("Expected TitleChanged event");
        }
    }

    #[tokio::test]
    async fn test_get_tabs_for_browser() {
        let monitor = TabMonitor::new();
        
        let mut browser_tabs = HashMap::new();
        browser_tabs.insert(BrowserType::Chrome, vec![
            create_test_tab("tab1", "https://example.com", "Chrome Tab", BrowserType::Chrome),
        ]);
        browser_tabs.insert(BrowserType::Firefox, vec![
            create_test_tab("tab2", "https://example.org", "Firefox Tab", BrowserType::Firefox),
        ]);
        monitor.update_tabs(browser_tabs).await;
        
        let chrome_tabs = monitor.get_tabs_for_browser(BrowserType::Chrome).await;
        let firefox_tabs = monitor.get_tabs_for_browser(BrowserType::Firefox).await;
        
        assert_eq!(chrome_tabs.len(), 1);
        assert_eq!(firefox_tabs.len(), 1);
        assert_eq!(chrome_tabs[0].title, "Chrome Tab");
        assert_eq!(firefox_tabs[0].title, "Firefox Tab");
    }

    #[tokio::test]
    async fn test_get_recent_events() {
        let monitor = TabMonitor::new();
        
        // Create multiple tabs
        let mut browser_tabs = HashMap::new();
        browser_tabs.insert(BrowserType::Chrome, vec![
            create_test_tab("tab1", "https://example.com", "Tab 1", BrowserType::Chrome),
            create_test_tab("tab2", "https://example.org", "Tab 2", BrowserType::Chrome),
        ]);
        monitor.update_tabs(browser_tabs).await;
        
        let events = monitor.get_recent_events(10).await;
        assert_eq!(events.len(), 2);
    }

    #[tokio::test]
    async fn test_skip_internal_pages() {
        let monitor = TabMonitor::new();
        
        let mut browser_tabs = HashMap::new();
        browser_tabs.insert(BrowserType::Chrome, vec![
            create_test_tab("tab1", "https://example.com", "Normal Tab", BrowserType::Chrome),
            create_test_tab("tab2", "chrome://newtab", "New Tab", BrowserType::Chrome),
        ]);
        let events = monitor.update_tabs(browser_tabs).await;
        
        // Only the normal tab should generate an event
        assert_eq!(events.len(), 1);
        if let TabEvent::Created { tab, .. } = &events[0] {
            assert_eq!(tab.url, "https://example.com");
        }
    }

    #[tokio::test]
    async fn test_include_internal_pages_when_configured() {
        let config = TabMonitorConfig {
            include_internal_pages: true,
            ..Default::default()
        };
        let monitor = TabMonitor::with_config(config);
        
        let mut browser_tabs = HashMap::new();
        browser_tabs.insert(BrowserType::Chrome, vec![
            create_test_tab("tab1", "https://example.com", "Normal Tab", BrowserType::Chrome),
            create_test_tab("tab2", "chrome://newtab", "New Tab", BrowserType::Chrome),
        ]);
        let events = monitor.update_tabs(browser_tabs).await;
        
        // Both tabs should generate events
        assert_eq!(events.len(), 2);
    }

    #[tokio::test]
    async fn test_get_stats() {
        let monitor = TabMonitor::new();
        
        let mut browser_tabs = HashMap::new();
        browser_tabs.insert(BrowserType::Chrome, vec![
            create_test_tab("tab1", "https://example.com", "Tab 1", BrowserType::Chrome),
            create_test_tab("tab2", "https://example.org", "Tab 2", BrowserType::Chrome),
        ]);
        browser_tabs.insert(BrowserType::Firefox, vec![
            create_test_tab("tab3", "https://example.net", "Tab 3", BrowserType::Firefox),
        ]);
        monitor.update_tabs(browser_tabs).await;
        
        let stats = monitor.get_stats().await;
        
        assert_eq!(stats.total_tabs, 3);
        assert_eq!(stats.tabs_by_browser.get(&BrowserType::Chrome), Some(&2));
        assert_eq!(stats.tabs_by_browser.get(&BrowserType::Firefox), Some(&1));
        assert_eq!(stats.total_events, 3);
    }

    #[tokio::test]
    async fn test_clear() {
        let monitor = TabMonitor::new();
        
        let mut browser_tabs = HashMap::new();
        browser_tabs.insert(BrowserType::Chrome, vec![
            create_test_tab("tab1", "https://example.com", "Tab 1", BrowserType::Chrome),
        ]);
        monitor.update_tabs(browser_tabs).await;
        
        assert!(!monitor.get_current_tabs().await.is_empty());
        
        monitor.clear().await;
        
        assert!(monitor.get_current_tabs().await.is_empty());
        assert!(monitor.get_recent_events(10).await.is_empty());
    }
}
