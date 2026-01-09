import 'dart:io';
import 'package:system_tray/system_tray.dart';
import 'package:window_manager/window_manager.dart';

/// Service for managing system tray functionality
class SystemTrayService {
  SystemTray? _systemTray;
  bool _initialized = false;
  
  Future<void> initialize() async {
    if (!Platform.isWindows && !Platform.isLinux && !Platform.isMacOS) {
      return;
    }
    
    _systemTray = SystemTray();
    
    // Initialize system tray with app icon
    await _systemTray!.initSystemTray(
      title: 'Web Page Manager',
      iconPath: _getIconPath(),
      toolTip: 'Web Page Manager - Click to open',
    );
    
    // Set up context menu
    final menu = Menu();
    await menu.buildFrom([
      MenuItemLabel(
        label: '打开主窗口',
        onClicked: (menuItem) => _showMainWindow(),
      ),
      MenuSeparator(),
      MenuItemLabel(
        label: '快速搜索',
        onClicked: (menuItem) => _showQuickSearch(),
      ),
      MenuItemLabel(
        label: '最近关闭的标签页',
        onClicked: (menuItem) => _showRecentlyClosed(),
      ),
      MenuSeparator(),
      MenuItemLabel(
        label: '设置',
        onClicked: (menuItem) => _showSettings(),
      ),
      MenuSeparator(),
      MenuItemLabel(
        label: '退出',
        onClicked: (menuItem) => _exitApp(),
      ),
    ]);
    
    await _systemTray!.setContextMenu(menu);
    
    // Handle tray icon click
    _systemTray!.registerSystemTrayEventHandler((eventName) {
      if (eventName == kSystemTrayEventClick) {
        _showMainWindow();
      } else if (eventName == kSystemTrayEventRightClick) {
        _systemTray!.popUpContextMenu();
      }
    });
    
    _initialized = true;
  }
  
  String _getIconPath() {
    if (Platform.isWindows) {
      return 'assets/icons/app_icon.ico';
    } else if (Platform.isMacOS) {
      return 'assets/icons/app_icon.png';
    } else {
      return 'assets/icons/app_icon.png';
    }
  }
  
  Future<void> _showMainWindow() async {
    await windowManager.show();
    await windowManager.focus();
  }
  
  Future<void> _showQuickSearch() async {
    await _showMainWindow();
    // TODO: Navigate to search screen
  }
  
  Future<void> _showRecentlyClosed() async {
    await _showMainWindow();
    // TODO: Navigate to history screen
  }
  
  Future<void> _showSettings() async {
    await _showMainWindow();
    // TODO: Navigate to settings screen
  }
  
  Future<void> _exitApp() async {
    await destroy();
    exit(0);
  }
  
  /// Update tray tooltip
  Future<void> updateTooltip(String tooltip) async {
    if (_initialized && _systemTray != null) {
      await _systemTray!.setToolTip(tooltip);
    }
  }
  
  /// Show notification badge (if supported)
  Future<void> showBadge(int count) async {
    if (_initialized && _systemTray != null) {
      // Update tooltip to show count
      await _systemTray!.setToolTip('Web Page Manager - $count 个新活动');
    }
  }
  
  /// Hide to system tray
  Future<void> hideToTray() async {
    await windowManager.hide();
  }
  
  /// Destroy system tray
  Future<void> destroy() async {
    if (_initialized && _systemTray != null) {
      await _systemTray!.destroy();
      _initialized = false;
    }
  }
  
  bool get isInitialized => _initialized;
}
