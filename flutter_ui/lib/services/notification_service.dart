import 'dart:io';

/// Notification urgency level
enum NotificationUrgency {
  low,
  normal,
  high,
  critical,
}

/// Notification configuration
class NotificationConfig {
  final String title;
  final String body;
  final String? subtitle;
  final String? icon;
  final NotificationUrgency urgency;
  final List<NotificationAction> actions;
  final Duration? timeout;
  
  const NotificationConfig({
    required this.title,
    required this.body,
    this.subtitle,
    this.icon,
    this.urgency = NotificationUrgency.normal,
    this.actions = const [],
    this.timeout,
  });
  
  /// Create a simple notification
  factory NotificationConfig.simple(String message) {
    return NotificationConfig(
      title: 'Web Page Manager',
      body: message,
      timeout: const Duration(seconds: 5),
    );
  }
  
  /// Create a notification with title
  factory NotificationConfig.withTitle(String title, String message) {
    return NotificationConfig(
      title: title,
      body: message,
      timeout: const Duration(seconds: 5),
    );
  }
}

/// Notification action button
class NotificationAction {
  final String id;
  final String label;
  
  const NotificationAction({
    required this.id,
    required this.label,
  });
}

/// Service for managing system notifications
/// 
/// This service provides cross-platform notification support for
/// Windows, Linux, and macOS. It integrates with the Rust backend
/// for actual notification display.
class NotificationService {
  bool _initialized = false;
  
  /// Initialize the notification service
  Future<void> initialize() async {
    if (!Platform.isWindows && !Platform.isLinux && !Platform.isMacOS) {
      return;
    }
    
    // TODO: Initialize via Rust backend
    // This would call CrossPlatformNotificationManager.initialize()
    
    _initialized = true;
  }
  
  /// Show a notification
  Future<String?> showNotification(NotificationConfig config) async {
    if (!_initialized) return null;
    
    // TODO: Call Rust backend to show notification
    // This would use the FFI bridge to call CrossPlatformNotificationManager
    
    return null; // Return notification ID
  }
  
  /// Show a simple notification
  Future<String?> showSimple(String message) async {
    return showNotification(NotificationConfig.simple(message));
  }
  
  /// Show a notification with title
  Future<String?> showWithTitle(String title, String message) async {
    return showNotification(NotificationConfig.withTitle(title, message));
  }
  
  /// Show notification for new tab activity
  Future<String?> showTabActivityNotification({
    required String browserName,
    required int tabCount,
  }) async {
    return showNotification(NotificationConfig(
      title: '标签页活动',
      body: '$browserName 中有 $tabCount 个新标签页',
      urgency: NotificationUrgency.normal,
      timeout: const Duration(seconds: 5),
    ));
  }
  
  /// Show notification for bookmark sync
  Future<String?> showBookmarkSyncNotification({
    required int syncedCount,
  }) async {
    return showNotification(NotificationConfig(
      title: '书签同步完成',
      body: '已同步 $syncedCount 个书签',
      urgency: NotificationUrgency.low,
      timeout: const Duration(seconds: 3),
    ));
  }
  
  /// Show notification for content analysis complete
  Future<String?> showAnalysisCompleteNotification({
    required int analyzedCount,
  }) async {
    return showNotification(NotificationConfig(
      title: '内容分析完成',
      body: '已分析 $analyzedCount 个页面',
      urgency: NotificationUrgency.low,
      actions: const [
        NotificationAction(id: 'view_results', label: '查看结果'),
      ],
      timeout: const Duration(seconds: 5),
    ));
  }
  
  /// Show notification for duplicate bookmarks found
  Future<String?> showDuplicatesFoundNotification({
    required int duplicateCount,
  }) async {
    return showNotification(NotificationConfig(
      title: '发现重复书签',
      body: '发现 $duplicateCount 组重复书签，点击查看详情',
      urgency: NotificationUrgency.normal,
      actions: const [
        NotificationAction(id: 'view_duplicates', label: '查看详情'),
        NotificationAction(id: 'dismiss', label: '稍后处理'),
      ],
    ));
  }
  
  bool get isInitialized => _initialized;
  
  /// Dispose the service
  Future<void> dispose() async {
    _initialized = false;
  }
}
