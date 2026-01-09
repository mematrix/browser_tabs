import 'dart:io';

/// Tray menu item types
abstract class TrayMenuItem {
  /// Create a clickable menu item
  static TrayMenuItemLabel item({
    required String id,
    required String label,
    bool enabled = true,
    bool? checked,
  }) {
    return TrayMenuItemLabel(
      id: id,
      label: label,
      enabled: enabled,
      checked: checked,
    );
  }
  
  /// Create a separator
  static TrayMenuSeparator separator() {
    return const TrayMenuSeparator();
  }
  
  /// Create a submenu
  static TrayMenuSubmenu submenu({
    required String id,
    required String label,
    required List<TrayMenuItem> items,
  }) {
    return TrayMenuSubmenu(
      id: id,
      label: label,
      items: items,
    );
  }
}

/// A clickable menu item
class TrayMenuItemLabel implements TrayMenuItem {
  final String id;
  final String label;
  final bool enabled;
  final bool? checked;
  
  const TrayMenuItemLabel({
    required this.id,
    required this.label,
    this.enabled = true,
    this.checked,
  });
}

/// A separator line
class TrayMenuSeparator implements TrayMenuItem {
  const TrayMenuSeparator();
}

/// A submenu
class TrayMenuSubmenu implements TrayMenuItem {
  final String id;
  final String label;
  final List<TrayMenuItem> items;
  
  const TrayMenuSubmenu({
    required this.id,
    required this.label,
    required this.items,
  });
}

/// Tray event types
enum TrayEventType {
  iconClicked,
  iconDoubleClicked,
  iconRightClicked,
  menuItemSelected,
}

/// Tray event
class TrayEvent {
  final TrayEventType type;
  final String? menuItemId;
  
  const TrayEvent({
    required this.type,
    this.menuItemId,
  });
}

/// Callback for tray events
typedef TrayEventCallback = void Function(TrayEvent event);

/// Service for managing system tray functionality
/// 
/// This service provides cross-platform system tray support for
/// Windows, Linux, and macOS. It integrates with the Rust backend
/// for actual tray management.
class SystemTrayService {
  bool _initialized = false;
  String _tooltip = 'Web Page Manager - 点击打开';
  List<TrayMenuItem> _menuItems = [];
  TrayEventCallback? _eventCallback;
  
  /// Initialize the system tray service
  Future<void> initialize() async {
    if (!Platform.isWindows && !Platform.isLinux && !Platform.isMacOS) {
      return;
    }
    
    // Set up default menu
    _menuItems = _getDefaultMenu();
    
    // TODO: Initialize via Rust backend
    // This would call CrossPlatformTrayManager.initialize()
    
    _initialized = true;
  }
  
  /// Get the default context menu
  List<TrayMenuItem> _getDefaultMenu() {
    return [
      TrayMenuItem.item(id: 'show_window', label: '打开主窗口'),
      TrayMenuItem.separator(),
      TrayMenuItem.item(id: 'quick_search', label: '快速搜索'),
      TrayMenuItem.item(id: 'recent_closed', label: '最近关闭的标签页'),
      TrayMenuItem.separator(),
      TrayMenuItem.item(id: 'settings', label: '设置'),
      TrayMenuItem.separator(),
      TrayMenuItem.item(id: 'exit', label: '退出'),
    ];
  }
  
  /// Set the context menu items
  Future<void> setMenu(List<TrayMenuItem> items) async {
    if (!_initialized) return;
    
    _menuItems = items;
    
    // TODO: Call Rust backend to update menu
  }
  
  /// Update the tooltip text
  Future<void> setTooltip(String tooltip) async {
    if (!_initialized) return;
    
    _tooltip = tooltip;
    
    // TODO: Call Rust backend to update tooltip
  }
  
  /// Update tooltip with activity count
  Future<void> setActivityBadge(int count) async {
    final tooltip = count > 0
        ? 'Web Page Manager - $count 个新活动'
        : 'Web Page Manager - 点击打开';
    await setTooltip(tooltip);
  }
  
  /// Set the event callback
  void setEventCallback(TrayEventCallback callback) {
    _eventCallback = callback;
  }
  
  /// Handle a tray event from the Rust backend
  void handleTrayEvent(TrayEvent event) {
    _eventCallback?.call(event);
  }
  
  /// Show the tray icon
  Future<void> show() async {
    if (!_initialized) return;
    
    // TODO: Call Rust backend to show tray
  }
  
  /// Hide the tray icon
  Future<void> hide() async {
    if (!_initialized) return;
    
    // TODO: Call Rust backend to hide tray
  }
  
  /// Hide the main window to tray
  Future<void> hideToTray() async {
    // TODO: Call window manager to hide window
    // Then show tray icon
    await show();
  }
  
  /// Restore from tray
  Future<void> restoreFromTray() async {
    // TODO: Call window manager to show window
  }
  
  /// Get the current tooltip
  String get tooltip => _tooltip;
  
  /// Get the current menu items
  List<TrayMenuItem> get menuItems => List.unmodifiable(_menuItems);
  
  bool get isInitialized => _initialized;
  
  /// Dispose the service
  Future<void> dispose() async {
    await hide();
    _initialized = false;
  }
}
