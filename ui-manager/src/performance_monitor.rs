//! Performance monitoring and resource management module
//!
//! This module provides:
//! - Application performance monitoring
//! - Resource usage tracking and optimization
//! - Adaptive resource management based on system load
//! - User settings and configuration management

use web_page_manager_core::*;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

/// Performance metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Timestamp of the metrics collection
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Memory usage in bytes
    pub memory_usage_bytes: u64,
    /// CPU usage percentage (0-100)
    pub cpu_usage_percent: f32,
    /// Number of active browser connections
    pub active_connections: usize,
    /// Number of pages being managed
    pub managed_pages: usize,
    /// Number of pending AI analysis tasks
    pub pending_ai_tasks: usize,
    /// Database size in bytes
    pub database_size_bytes: u64,
    /// Cache hit rate (0-1)
    pub cache_hit_rate: f32,
    /// Average response time in milliseconds
    pub avg_response_time_ms: u64,
    /// Number of errors in the last minute
    pub recent_error_count: usize,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            timestamp: chrono::Utc::now(),
            memory_usage_bytes: 0,
            cpu_usage_percent: 0.0,
            active_connections: 0,
            managed_pages: 0,
            pending_ai_tasks: 0,
            database_size_bytes: 0,
            cache_hit_rate: 0.0,
            avg_response_time_ms: 0,
            recent_error_count: 0,
        }
    }
}

/// Resource usage level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ResourceLevel {
    /// Low resource usage - can increase processing
    Low,
    /// Normal resource usage
    #[default]
    Normal,
    /// High resource usage - should reduce processing
    High,
    /// Critical resource usage - must reduce processing immediately
    Critical,
}

/// Processing priority level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessingPriority {
    /// Background processing - lowest priority
    Background,
    /// Normal processing
    Normal,
    /// High priority - user-initiated actions
    High,
    /// Critical - must complete immediately
    Critical,
}

/// Resource management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConfig {
    /// Maximum memory usage in MB before throttling
    pub max_memory_mb: u64,
    /// Maximum CPU usage percentage before throttling
    pub max_cpu_percent: f32,
    /// Maximum number of concurrent AI tasks
    pub max_concurrent_ai_tasks: usize,
    /// Maximum database size in MB
    pub max_database_size_mb: u64,
    /// Enable adaptive resource management
    pub adaptive_management: bool,
    /// Background processing interval in seconds
    pub background_interval_secs: u64,
    /// Cache size limit in MB
    pub cache_size_mb: u64,
}

impl Default for ResourceConfig {
    fn default() -> Self {
        Self {
            max_memory_mb: 512,
            max_cpu_percent: 50.0,
            max_concurrent_ai_tasks: 4,
            max_database_size_mb: 1024,
            adaptive_management: true,
            background_interval_secs: 30,
            cache_size_mb: 100,
        }
    }
}


/// Application settings configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// Theme mode
    pub theme_mode: ThemeMode,
    /// Minimize to tray when closing window
    pub minimize_to_tray: bool,
    /// Show system notifications
    pub show_notifications: bool,
    /// Enable global hotkeys
    pub enable_hotkeys: bool,
    /// Auto refresh data
    pub auto_refresh: bool,
    /// Auto refresh interval in seconds
    pub auto_refresh_interval_secs: u32,
    /// Default browser for opening links
    pub default_browser: String,
    /// Resource management configuration
    pub resource_config: ResourceConfig,
    /// Enable performance monitoring
    pub enable_performance_monitoring: bool,
    /// Performance history retention in hours
    pub performance_history_hours: u32,
}

/// Theme mode setting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ThemeMode {
    Light,
    Dark,
    #[default]
    System,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme_mode: ThemeMode::System,
            minimize_to_tray: true,
            show_notifications: true,
            enable_hotkeys: true,
            auto_refresh: true,
            auto_refresh_interval_secs: 30,
            default_browser: "chrome".to_string(),
            resource_config: ResourceConfig::default(),
            enable_performance_monitoring: true,
            performance_history_hours: 24,
        }
    }
}

/// Performance monitor for tracking application metrics
pub struct PerformanceMonitor {
    /// Current metrics
    current_metrics: Arc<RwLock<PerformanceMetrics>>,
    /// Historical metrics (circular buffer)
    metrics_history: Arc<RwLock<VecDeque<PerformanceMetrics>>>,
    /// Maximum history size
    max_history_size: usize,
    /// Resource configuration
    config: Arc<RwLock<ResourceConfig>>,
    /// Current resource level
    resource_level: Arc<RwLock<ResourceLevel>>,
    /// Current processing priority
    processing_priority: Arc<RwLock<ProcessingPriority>>,
    /// Whether monitoring is active
    is_monitoring: std::sync::atomic::AtomicBool,
    /// Response time samples for averaging
    response_times: Arc<RwLock<VecDeque<u64>>>,
    /// Error timestamps for counting recent errors
    error_timestamps: Arc<RwLock<VecDeque<Instant>>>,
    /// Cache statistics
    cache_stats: Arc<RwLock<CacheStats>>,
}

/// Cache statistics
#[derive(Debug, Clone, Default)]
struct CacheStats {
    hits: u64,
    misses: u64,
}

impl PerformanceMonitor {
    /// Create a new performance monitor
    pub fn new() -> Self {
        Self {
            current_metrics: Arc::new(RwLock::new(PerformanceMetrics::default())),
            metrics_history: Arc::new(RwLock::new(VecDeque::with_capacity(1000))),
            max_history_size: 1000,
            config: Arc::new(RwLock::new(ResourceConfig::default())),
            resource_level: Arc::new(RwLock::new(ResourceLevel::Normal)),
            processing_priority: Arc::new(RwLock::new(ProcessingPriority::Normal)),
            is_monitoring: std::sync::atomic::AtomicBool::new(false),
            response_times: Arc::new(RwLock::new(VecDeque::with_capacity(100))),
            error_timestamps: Arc::new(RwLock::new(VecDeque::with_capacity(100))),
            cache_stats: Arc::new(RwLock::new(CacheStats::default())),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: ResourceConfig) -> Self {
        let mut monitor = Self::new();
        monitor.config = Arc::new(RwLock::new(config));
        monitor
    }

    /// Start performance monitoring
    pub async fn start_monitoring(&self) -> Result<()> {
        tracing::info!("Starting performance monitoring");
        self.is_monitoring.store(true, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    /// Stop performance monitoring
    pub async fn stop_monitoring(&self) -> Result<()> {
        tracing::info!("Stopping performance monitoring");
        self.is_monitoring.store(false, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    /// Check if monitoring is active
    pub fn is_monitoring(&self) -> bool {
        self.is_monitoring.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Update resource configuration
    pub async fn update_config(&self, config: ResourceConfig) -> Result<()> {
        let mut current = self.config.write().await;
        *current = config;
        tracing::info!("Resource configuration updated");
        Ok(())
    }

    /// Get current configuration
    pub async fn get_config(&self) -> ResourceConfig {
        self.config.read().await.clone()
    }

    /// Record a response time sample
    pub async fn record_response_time(&self, duration_ms: u64) {
        let mut times = self.response_times.write().await;
        if times.len() >= 100 {
            times.pop_front();
        }
        times.push_back(duration_ms);
    }

    /// Record an error occurrence
    pub async fn record_error(&self) {
        let mut errors = self.error_timestamps.write().await;
        let now = Instant::now();
        // Remove errors older than 1 minute
        while let Some(front) = errors.front() {
            if now.duration_since(*front) > Duration::from_secs(60) {
                errors.pop_front();
            } else {
                break;
            }
        }
        errors.push_back(now);
    }

    /// Record a cache hit
    pub async fn record_cache_hit(&self) {
        let mut stats = self.cache_stats.write().await;
        stats.hits += 1;
    }

    /// Record a cache miss
    pub async fn record_cache_miss(&self) {
        let mut stats = self.cache_stats.write().await;
        stats.misses += 1;
    }


    /// Collect current performance metrics
    pub async fn collect_metrics(&self) -> PerformanceMetrics {
        let response_times = self.response_times.read().await;
        let avg_response_time = if response_times.is_empty() {
            0
        } else {
            response_times.iter().sum::<u64>() / response_times.len() as u64
        };

        let error_timestamps = self.error_timestamps.read().await;
        let now = Instant::now();
        let recent_errors = error_timestamps
            .iter()
            .filter(|t| now.duration_since(**t) <= Duration::from_secs(60))
            .count();

        let cache_stats = self.cache_stats.read().await;
        let cache_hit_rate = if cache_stats.hits + cache_stats.misses > 0 {
            cache_stats.hits as f32 / (cache_stats.hits + cache_stats.misses) as f32
        } else {
            0.0
        };

        // Get system memory usage
        let memory_usage = Self::get_process_memory();
        let cpu_usage = Self::get_cpu_usage();

        let metrics = PerformanceMetrics {
            timestamp: chrono::Utc::now(),
            memory_usage_bytes: memory_usage,
            cpu_usage_percent: cpu_usage,
            active_connections: 0, // Will be updated by browser connector
            managed_pages: 0,      // Will be updated by page manager
            pending_ai_tasks: 0,   // Will be updated by AI processor
            database_size_bytes: 0, // Will be updated by data access layer
            cache_hit_rate,
            avg_response_time_ms: avg_response_time,
            recent_error_count: recent_errors,
        };

        // Update current metrics
        {
            let mut current = self.current_metrics.write().await;
            *current = metrics.clone();
        }

        // Add to history
        {
            let mut history = self.metrics_history.write().await;
            if history.len() >= self.max_history_size {
                history.pop_front();
            }
            history.push_back(metrics.clone());
        }

        // Update resource level based on metrics
        self.update_resource_level(&metrics).await;

        metrics
    }

    /// Get current metrics without collecting new ones
    pub async fn get_current_metrics(&self) -> PerformanceMetrics {
        self.current_metrics.read().await.clone()
    }

    /// Get metrics history
    pub async fn get_metrics_history(&self) -> Vec<PerformanceMetrics> {
        self.metrics_history.read().await.iter().cloned().collect()
    }

    /// Get current resource level
    pub async fn get_resource_level(&self) -> ResourceLevel {
        *self.resource_level.read().await
    }

    /// Get current processing priority
    pub async fn get_processing_priority(&self) -> ProcessingPriority {
        *self.processing_priority.read().await
    }

    /// Update resource level based on current metrics
    async fn update_resource_level(&self, metrics: &PerformanceMetrics) {
        let config = self.config.read().await;
        
        let memory_percent = (metrics.memory_usage_bytes as f64 / (config.max_memory_mb * 1024 * 1024) as f64) * 100.0;
        let cpu_percent = metrics.cpu_usage_percent;

        let new_level = if memory_percent > 90.0 || cpu_percent > 90.0 {
            ResourceLevel::Critical
        } else if memory_percent > 75.0 || cpu_percent > 75.0 {
            ResourceLevel::High
        } else if memory_percent > 50.0 || cpu_percent > 50.0 {
            ResourceLevel::Normal
        } else {
            ResourceLevel::Low
        };

        let mut level = self.resource_level.write().await;
        if *level != new_level {
            tracing::info!("Resource level changed from {:?} to {:?}", *level, new_level);
            *level = new_level;
        }

        // Adjust processing priority based on resource level
        if config.adaptive_management {
            let new_priority = match new_level {
                ResourceLevel::Critical => ProcessingPriority::Background,
                ResourceLevel::High => ProcessingPriority::Normal,
                ResourceLevel::Normal => ProcessingPriority::Normal,
                ResourceLevel::Low => ProcessingPriority::High,
            };

            let mut priority = self.processing_priority.write().await;
            if *priority != new_priority {
                tracing::info!("Processing priority adjusted from {:?} to {:?}", *priority, new_priority);
                *priority = new_priority;
            }
        }
    }

    /// Check if a task should be throttled based on current resource level
    pub async fn should_throttle(&self, task_priority: ProcessingPriority) -> bool {
        let resource_level = self.get_resource_level().await;
        let current_priority = self.get_processing_priority().await;

        match resource_level {
            ResourceLevel::Critical => task_priority == ProcessingPriority::Background,
            ResourceLevel::High => {
                task_priority == ProcessingPriority::Background 
                    && current_priority != ProcessingPriority::Critical
            }
            ResourceLevel::Normal | ResourceLevel::Low => false,
        }
    }

    /// Get recommended delay for background tasks based on resource level
    pub async fn get_recommended_delay(&self) -> Duration {
        let resource_level = self.get_resource_level().await;
        match resource_level {
            ResourceLevel::Critical => Duration::from_secs(30),
            ResourceLevel::High => Duration::from_secs(10),
            ResourceLevel::Normal => Duration::from_secs(5),
            ResourceLevel::Low => Duration::from_secs(1),
        }
    }

    /// Get process memory usage in bytes
    fn get_process_memory() -> u64 {
        #[cfg(target_os = "linux")]
        {
            if let Ok(content) = std::fs::read_to_string("/proc/self/statm") {
                if let Some(rss) = content.split_whitespace().nth(1) {
                    if let Ok(pages) = rss.parse::<u64>() {
                        return pages * 4096; // Page size is typically 4KB
                    }
                }
            }
            0
        }

        #[cfg(target_os = "windows")]
        {
            // Windows implementation would use GetProcessMemoryInfo
            0
        }

        #[cfg(target_os = "macos")]
        {
            // macOS implementation would use task_info
            0
        }

        #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
        {
            0
        }
    }

    /// Get CPU usage percentage
    fn get_cpu_usage() -> f32 {
        // This is a simplified implementation
        // A real implementation would track CPU time over intervals
        0.0
    }

    /// Clear metrics history
    pub async fn clear_history(&self) {
        let mut history = self.metrics_history.write().await;
        history.clear();
        tracing::info!("Performance metrics history cleared");
    }

    /// Get performance summary
    pub async fn get_summary(&self) -> PerformanceSummary {
        let history = self.metrics_history.read().await;
        let current = self.current_metrics.read().await;
        let config = self.config.read().await;

        if history.is_empty() {
            return PerformanceSummary::default();
        }

        let avg_memory = history.iter().map(|m| m.memory_usage_bytes).sum::<u64>() / history.len() as u64;
        let avg_cpu = history.iter().map(|m| m.cpu_usage_percent).sum::<f32>() / history.len() as f32;
        let avg_response = history.iter().map(|m| m.avg_response_time_ms).sum::<u64>() / history.len() as u64;
        let total_errors = history.iter().map(|m| m.recent_error_count).sum::<usize>();

        let max_memory = history.iter().map(|m| m.memory_usage_bytes).max().unwrap_or(0);
        let max_cpu = history.iter().map(|m| m.cpu_usage_percent).fold(0.0f32, f32::max);

        let resource_level = *self.resource_level.read().await;

        PerformanceSummary {
            current_memory_mb: current.memory_usage_bytes / (1024 * 1024),
            current_cpu_percent: current.cpu_usage_percent,
            avg_memory_mb: avg_memory / (1024 * 1024),
            avg_cpu_percent: avg_cpu,
            max_memory_mb: max_memory / (1024 * 1024),
            max_cpu_percent: max_cpu,
            avg_response_time_ms: avg_response,
            total_errors,
            cache_hit_rate: current.cache_hit_rate,
            resource_level,
            memory_limit_mb: config.max_memory_mb,
            cpu_limit_percent: config.max_cpu_percent,
            samples_count: history.len(),
        }
    }
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}


/// Performance summary for display
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PerformanceSummary {
    /// Current memory usage in MB
    pub current_memory_mb: u64,
    /// Current CPU usage percentage
    pub current_cpu_percent: f32,
    /// Average memory usage in MB
    pub avg_memory_mb: u64,
    /// Average CPU usage percentage
    pub avg_cpu_percent: f32,
    /// Maximum memory usage in MB
    pub max_memory_mb: u64,
    /// Maximum CPU usage percentage
    pub max_cpu_percent: f32,
    /// Average response time in milliseconds
    pub avg_response_time_ms: u64,
    /// Total errors in history
    pub total_errors: usize,
    /// Cache hit rate
    pub cache_hit_rate: f32,
    /// Current resource level
    pub resource_level: ResourceLevel,
    /// Memory limit in MB
    pub memory_limit_mb: u64,
    /// CPU limit percentage
    pub cpu_limit_percent: f32,
    /// Number of samples in history
    pub samples_count: usize,
}

/// Settings manager for persisting application settings
pub struct SettingsManager {
    /// Current settings
    settings: Arc<RwLock<AppSettings>>,
    /// Settings file path
    settings_path: std::path::PathBuf,
}

impl SettingsManager {
    /// Create a new settings manager
    pub fn new() -> Self {
        let settings_path = Self::get_default_settings_path();
        Self {
            settings: Arc::new(RwLock::new(AppSettings::default())),
            settings_path,
        }
    }

    /// Create with custom path
    pub fn with_path(path: impl Into<std::path::PathBuf>) -> Self {
        Self {
            settings: Arc::new(RwLock::new(AppSettings::default())),
            settings_path: path.into(),
        }
    }

    /// Get default settings path
    fn get_default_settings_path() -> std::path::PathBuf {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        config_dir.join("web-page-manager").join("settings.json")
    }

    /// Load settings from file
    pub async fn load(&self) -> Result<()> {
        if !self.settings_path.exists() {
            tracing::info!("Settings file not found, using defaults");
            return Ok(());
        }

        let content = tokio::fs::read_to_string(&self.settings_path).await
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::IO { source: e },
            })?;

        let loaded_settings: AppSettings = serde_json::from_str(&content)
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Serialization { source: e },
            })?;

        let mut settings = self.settings.write().await;
        *settings = loaded_settings;
        tracing::info!("Settings loaded from {:?}", self.settings_path);
        Ok(())
    }

    /// Save settings to file
    pub async fn save(&self) -> Result<()> {
        // Ensure directory exists
        if let Some(parent) = self.settings_path.parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| WebPageManagerError::System {
                    source: SystemError::IO { source: e },
                })?;
        }

        let settings = self.settings.read().await;
        let content = serde_json::to_string_pretty(&*settings)
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Serialization { source: e },
            })?;

        tokio::fs::write(&self.settings_path, content).await
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::IO { source: e },
            })?;

        tracing::info!("Settings saved to {:?}", self.settings_path);
        Ok(())
    }

    /// Get current settings
    pub async fn get(&self) -> AppSettings {
        self.settings.read().await.clone()
    }

    /// Update settings
    pub async fn update(&self, settings: AppSettings) -> Result<()> {
        {
            let mut current = self.settings.write().await;
            *current = settings;
        }
        self.save().await
    }

    /// Update a single setting
    pub async fn update_theme(&self, theme: ThemeMode) -> Result<()> {
        {
            let mut settings = self.settings.write().await;
            settings.theme_mode = theme;
        }
        self.save().await
    }

    /// Update minimize to tray setting
    pub async fn update_minimize_to_tray(&self, value: bool) -> Result<()> {
        {
            let mut settings = self.settings.write().await;
            settings.minimize_to_tray = value;
        }
        self.save().await
    }

    /// Update notifications setting
    pub async fn update_notifications(&self, value: bool) -> Result<()> {
        {
            let mut settings = self.settings.write().await;
            settings.show_notifications = value;
        }
        self.save().await
    }

    /// Update hotkeys setting
    pub async fn update_hotkeys(&self, value: bool) -> Result<()> {
        {
            let mut settings = self.settings.write().await;
            settings.enable_hotkeys = value;
        }
        self.save().await
    }

    /// Update auto refresh settings
    pub async fn update_auto_refresh(&self, enabled: bool, interval_secs: u32) -> Result<()> {
        {
            let mut settings = self.settings.write().await;
            settings.auto_refresh = enabled;
            settings.auto_refresh_interval_secs = interval_secs;
        }
        self.save().await
    }

    /// Update default browser
    pub async fn update_default_browser(&self, browser: String) -> Result<()> {
        {
            let mut settings = self.settings.write().await;
            settings.default_browser = browser;
        }
        self.save().await
    }

    /// Update resource configuration
    pub async fn update_resource_config(&self, config: ResourceConfig) -> Result<()> {
        {
            let mut settings = self.settings.write().await;
            settings.resource_config = config;
        }
        self.save().await
    }

    /// Reset to defaults
    pub async fn reset_to_defaults(&self) -> Result<()> {
        {
            let mut settings = self.settings.write().await;
            *settings = AppSettings::default();
        }
        self.save().await
    }
}

impl Default for SettingsManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_performance_monitor_creation() {
        let monitor = PerformanceMonitor::new();
        assert!(!monitor.is_monitoring());
    }

    #[tokio::test]
    async fn test_performance_monitor_start_stop() {
        let monitor = PerformanceMonitor::new();
        
        monitor.start_monitoring().await.unwrap();
        assert!(monitor.is_monitoring());
        
        monitor.stop_monitoring().await.unwrap();
        assert!(!monitor.is_monitoring());
    }

    #[tokio::test]
    async fn test_record_response_time() {
        let monitor = PerformanceMonitor::new();
        
        monitor.record_response_time(100).await;
        monitor.record_response_time(200).await;
        monitor.record_response_time(150).await;
        
        let metrics = monitor.collect_metrics().await;
        assert_eq!(metrics.avg_response_time_ms, 150);
    }

    #[tokio::test]
    async fn test_record_error() {
        let monitor = PerformanceMonitor::new();
        
        monitor.record_error().await;
        monitor.record_error().await;
        
        let metrics = monitor.collect_metrics().await;
        assert_eq!(metrics.recent_error_count, 2);
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let monitor = PerformanceMonitor::new();
        
        monitor.record_cache_hit().await;
        monitor.record_cache_hit().await;
        monitor.record_cache_miss().await;
        
        let metrics = monitor.collect_metrics().await;
        // 2 hits / 3 total = 0.666...
        assert!(metrics.cache_hit_rate > 0.6 && metrics.cache_hit_rate < 0.7);
    }

    #[tokio::test]
    async fn test_resource_config_update() {
        let monitor = PerformanceMonitor::new();
        
        let new_config = ResourceConfig {
            max_memory_mb: 1024,
            max_cpu_percent: 75.0,
            ..Default::default()
        };
        
        monitor.update_config(new_config.clone()).await.unwrap();
        let config = monitor.get_config().await;
        
        assert_eq!(config.max_memory_mb, 1024);
        assert_eq!(config.max_cpu_percent, 75.0);
    }

    #[tokio::test]
    async fn test_metrics_history() {
        let monitor = PerformanceMonitor::new();
        
        // Collect metrics multiple times
        monitor.collect_metrics().await;
        monitor.collect_metrics().await;
        monitor.collect_metrics().await;
        
        let history = monitor.get_metrics_history().await;
        assert_eq!(history.len(), 3);
    }

    #[tokio::test]
    async fn test_clear_history() {
        let monitor = PerformanceMonitor::new();
        
        monitor.collect_metrics().await;
        monitor.collect_metrics().await;
        
        monitor.clear_history().await;
        
        let history = monitor.get_metrics_history().await;
        assert!(history.is_empty());
    }

    #[tokio::test]
    async fn test_should_throttle() {
        let monitor = PerformanceMonitor::new();
        
        // At normal level, should not throttle
        let should_throttle = monitor.should_throttle(ProcessingPriority::Background).await;
        assert!(!should_throttle);
    }

    #[tokio::test]
    async fn test_recommended_delay() {
        let monitor = PerformanceMonitor::new();
        
        let delay = monitor.get_recommended_delay().await;
        // At normal level, delay should be 5 seconds
        assert_eq!(delay, Duration::from_secs(5));
    }

    #[test]
    fn test_app_settings_default() {
        let settings = AppSettings::default();
        
        assert_eq!(settings.theme_mode, ThemeMode::System);
        assert!(settings.minimize_to_tray);
        assert!(settings.show_notifications);
        assert!(settings.enable_hotkeys);
        assert!(settings.auto_refresh);
        assert_eq!(settings.auto_refresh_interval_secs, 30);
        assert_eq!(settings.default_browser, "chrome");
    }

    #[test]
    fn test_resource_config_default() {
        let config = ResourceConfig::default();
        
        assert_eq!(config.max_memory_mb, 512);
        assert_eq!(config.max_cpu_percent, 50.0);
        assert_eq!(config.max_concurrent_ai_tasks, 4);
        assert!(config.adaptive_management);
    }

    #[tokio::test]
    async fn test_performance_summary() {
        let monitor = PerformanceMonitor::new();
        
        // Record some data
        monitor.record_response_time(100).await;
        monitor.record_cache_hit().await;
        monitor.collect_metrics().await;
        
        let summary = monitor.get_summary().await;
        assert_eq!(summary.samples_count, 1);
    }
}
