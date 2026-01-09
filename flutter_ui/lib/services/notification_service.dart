import 'dart:io';
import 'package:local_notifier/local_notifier.dart';

/// Service for managing system notifications
class NotificationService {
  bool _initialized = false;
  
  Future<void> initialize() async {
    if (!Platform.isWindows && !Platform.isLinux && !Platform.isMacOS) {
      return;
    }
    
    await localNotifier.setup(
      appName: 'Web Page Manager',
      shortcutPolicy: ShortcutPolicy.requireCreate,
    );
    
    _initialized = true;
  }
  
  /// Show a simple notification
  Future<void> showNotification({
    required String title,
    required String body,
    String? subtitle,
  }) async {
    if (!_initialized) return;
    
    final notification = LocalNotification(
      title: title,
      body: body,
      subtitle: subtitle,
    );
    
    notification.onShow = () {
      // Notification shown
    };
    
    notification.onClose = (reason) {
      // Notification closed
    };
    
    notification.onClick = () {
      // Notification clicked - could open app or specific screen
    };
    
    await notification.show();
  }
  
  /// Show notification for new tab activity
  Future<void> showTabActivityNotification({
    required String browserName,
    required int tabCount,
  }) async {
    await showNotification(
      title: '标签页活动',
      body: '$browserName 中有 $tabCount 个新标签页',
    );
  }
  
  /// Show notification for bookmark sync
  Future<void> showBookmarkSyncNotification({
    required int syncedCount,
  }) async {
    await showNotification(
      title: '书签同步完成',
      body: '已同步 $syncedCount 个书签',
    );
  }
  
  /// Show notification for content analysis complete
  Future<void> showAnalysisCompleteNotification({
    required int analyzedCount,
  }) async {
    await showNotification(
      title: '内容分析完成',
      body: '已分析 $analyzedCount 个页面',
    );
  }
  
  /// Show notification for duplicate bookmarks found
  Future<void> showDuplicatesFoundNotification({
    required int duplicateCount,
  }) async {
    await showNotification(
      title: '发现重复书签',
      body: '发现 $duplicateCount 组重复书签，点击查看详情',
    );
  }
  
  bool get isInitialized => _initialized;
}
