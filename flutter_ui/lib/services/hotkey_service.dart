import 'dart:io';
import 'package:hotkey_manager/hotkey_manager.dart';
import 'package:flutter/services.dart';

/// Callback type for hotkey actions
typedef HotkeyCallback = void Function();

/// Service for managing global hotkeys
class HotkeyService {
  bool _initialized = false;
  final Map<String, HotKey> _registeredHotkeys = {};
  final Map<String, HotkeyCallback> _callbacks = {};
  
  Future<void> initialize() async {
    if (!Platform.isWindows && !Platform.isLinux && !Platform.isMacOS) {
      return;
    }
    
    await hotKeyManager.unregisterAll();
    _initialized = true;
  }
  
  /// Register a global hotkey
  Future<bool> registerHotkey({
    required String id,
    required List<HotKeyModifier> modifiers,
    required KeyCode keyCode,
    required HotkeyCallback callback,
  }) async {
    if (!_initialized) return false;
    
    try {
      final hotKey = HotKey(
        key: keyCode,
        modifiers: modifiers,
        scope: HotKeyScope.system,
      );
      
      await hotKeyManager.register(
        hotKey,
        keyDownHandler: (hotKey) {
          _callbacks[id]?.call();
        },
      );
      
      _registeredHotkeys[id] = hotKey;
      _callbacks[id] = callback;
      
      return true;
    } catch (e) {
      return false;
    }
  }
  
  /// Unregister a hotkey
  Future<void> unregisterHotkey(String id) async {
    if (!_initialized) return;
    
    final hotKey = _registeredHotkeys[id];
    if (hotKey != null) {
      await hotKeyManager.unregister(hotKey);
      _registeredHotkeys.remove(id);
      _callbacks.remove(id);
    }
  }
  
  /// Register default hotkeys
  Future<void> registerDefaultHotkeys({
    required HotkeyCallback onQuickSearch,
    required HotkeyCallback onShowWindow,
    required HotkeyCallback onNewTab,
  }) async {
    // Ctrl+Shift+F - Quick search
    await registerHotkey(
      id: 'quick_search',
      modifiers: [HotKeyModifier.control, HotKeyModifier.shift],
      keyCode: KeyCode.keyF,
      callback: onQuickSearch,
    );
    
    // Ctrl+Shift+W - Show window
    await registerHotkey(
      id: 'show_window',
      modifiers: [HotKeyModifier.control, HotKeyModifier.shift],
      keyCode: KeyCode.keyW,
      callback: onShowWindow,
    );
    
    // Ctrl+Shift+T - New tab (in default browser)
    await registerHotkey(
      id: 'new_tab',
      modifiers: [HotKeyModifier.control, HotKeyModifier.shift],
      keyCode: KeyCode.keyT,
      callback: onNewTab,
    );
  }
  
  /// Unregister all hotkeys
  Future<void> unregisterAll() async {
    if (!_initialized) return;
    
    await hotKeyManager.unregisterAll();
    _registeredHotkeys.clear();
    _callbacks.clear();
  }
  
  /// Get list of registered hotkey IDs
  List<String> get registeredHotkeyIds => _registeredHotkeys.keys.toList();
  
  bool get isInitialized => _initialized;
}
