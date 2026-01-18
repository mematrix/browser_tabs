import 'dart:io';

/// Callback type for hotkey actions
typedef HotkeyCallback = void Function();

/// Hotkey definition
class HotkeyDefinition {
  final String id;
  final String keyCombination;
  final String action;
  final String description;

  const HotkeyDefinition({
    required this.id,
    required this.keyCombination,
    required this.action,
    required this.description,
  });
}

/// Service for managing global hotkeys
///
/// This service provides cross-platform global hotkey support for
/// Windows, Linux, and macOS. It integrates with the Rust backend
/// for actual hotkey registration.
class HotkeyService {
  bool _initialized = false;
  final Map<String, HotkeyDefinition> _registeredHotkeys = {};
  final Map<String, HotkeyCallback> _callbacks = {};

  /// Initialize the hotkey service
  Future<void> initialize() async {
    if (!Platform.isWindows && !Platform.isLinux && !Platform.isMacOS) {
      return;
    }

    // Clear any existing registrations
    _registeredHotkeys.clear();
    _callbacks.clear();

    _initialized = true;
  }

  /// Register a global hotkey
  ///
  /// Returns true if registration was successful.
  Future<bool> registerHotkey({
    required String id,
    required String keyCombination,
    required String action,
    required String description,
    required HotkeyCallback callback,
  }) async {
    if (!_initialized) return false;

    try {
      final hotkey = HotkeyDefinition(
        id: id,
        keyCombination: keyCombination,
        action: action,
        description: description,
      );

      _registeredHotkeys[id] = hotkey;
      _callbacks[id] = callback;

      // TODO: Call Rust backend to register the hotkey
      // This would use the FFI bridge to call CrossPlatformHotkeyManager

      return true;
    } catch (e) {
      return false;
    }
  }

  /// Unregister a hotkey
  Future<void> unregisterHotkey(String id) async {
    if (!_initialized) return;

    _registeredHotkeys.remove(id);
    _callbacks.remove(id);

    // TODO: Call Rust backend to unregister the hotkey
  }

  /// Register default hotkeys for the application
  Future<void> registerDefaultHotkeys({
    required HotkeyCallback onQuickSearch,
    required HotkeyCallback onShowWindow,
    required HotkeyCallback onNewTab,
  }) async {
    // Ctrl+Shift+F - Quick search
    await registerHotkey(
      id: 'quick_search',
      keyCombination: 'Ctrl+Shift+F',
      action: 'quick_search',
      description: '打开快速搜索',
      callback: onQuickSearch,
    );

    // Ctrl+Shift+W - Show window
    await registerHotkey(
      id: 'show_window',
      keyCombination: 'Ctrl+Shift+W',
      action: 'show_window',
      description: '显示主窗口',
      callback: onShowWindow,
    );

    // Ctrl+Shift+T - New tab (in default browser)
    await registerHotkey(
      id: 'new_tab',
      keyCombination: 'Ctrl+Shift+T',
      action: 'new_tab',
      description: '在默认浏览器中打开新标签页',
      callback: onNewTab,
    );
  }

  /// Unregister all hotkeys
  Future<void> unregisterAll() async {
    if (!_initialized) return;

    _registeredHotkeys.clear();
    _callbacks.clear();

    // TODO: Call Rust backend to unregister all hotkeys
  }

  /// Handle a hotkey press event from the Rust backend
  void handleHotkeyPressed(String hotkeyId) {
    final callback = _callbacks[hotkeyId];
    if (callback != null) {
      callback();
    }
  }

  /// Get list of registered hotkey IDs
  List<String> get registeredHotkeyIds => _registeredHotkeys.keys.toList();

  /// Get all registered hotkeys
  List<HotkeyDefinition> get registeredHotkeys =>
      _registeredHotkeys.values.toList();

  /// Check if a hotkey is registered
  bool isHotkeyRegistered(String id) => _registeredHotkeys.containsKey(id);

  bool get isInitialized => _initialized;

  /// Dispose the service
  Future<void> dispose() async {
    await unregisterAll();
    _initialized = false;
  }
}
