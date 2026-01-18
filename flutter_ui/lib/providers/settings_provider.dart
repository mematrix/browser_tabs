import 'package:flutter/material.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../models/performance_metrics.dart';

/// Provider for managing application settings
class SettingsProvider extends ChangeNotifier {
  ThemeMode _themeMode = ThemeMode.system;
  bool _minimizeToTray = true;
  bool _showNotifications = true;
  bool _enableHotkeys = true;
  bool _autoRefresh = true;
  int _autoRefreshInterval = 30; // seconds
  String _defaultBrowser = 'chrome';

  // Performance monitoring settings
  bool _enablePerformanceMonitoring = true;
  int _performanceHistoryHours = 24;
  ResourceConfig _resourceConfig = ResourceConfig.defaults();

  SettingsProvider() {
    _loadSettings();
  }

  // Getters
  ThemeMode get themeMode => _themeMode;
  bool get minimizeToTray => _minimizeToTray;
  bool get showNotifications => _showNotifications;
  bool get enableHotkeys => _enableHotkeys;
  bool get autoRefresh => _autoRefresh;
  int get autoRefreshInterval => _autoRefreshInterval;
  String get defaultBrowser => _defaultBrowser;
  bool get enablePerformanceMonitoring => _enablePerformanceMonitoring;
  int get performanceHistoryHours => _performanceHistoryHours;
  ResourceConfig get resourceConfig => _resourceConfig;

  /// Load settings from storage
  Future<void> _loadSettings() async {
    final prefs = await SharedPreferences.getInstance();

    final themeModeIndex = prefs.getInt('themeMode') ?? 0;
    _themeMode = ThemeMode.values[themeModeIndex];

    _minimizeToTray = prefs.getBool('minimizeToTray') ?? true;
    _showNotifications = prefs.getBool('showNotifications') ?? true;
    _enableHotkeys = prefs.getBool('enableHotkeys') ?? true;
    _autoRefresh = prefs.getBool('autoRefresh') ?? true;
    _autoRefreshInterval = prefs.getInt('autoRefreshInterval') ?? 30;
    _defaultBrowser = prefs.getString('defaultBrowser') ?? 'chrome';

    // Performance settings
    _enablePerformanceMonitoring =
        prefs.getBool('enablePerformanceMonitoring') ?? true;
    _performanceHistoryHours = prefs.getInt('performanceHistoryHours') ?? 24;

    // Resource config
    _resourceConfig = ResourceConfig(
      maxMemoryMb: prefs.getInt('maxMemoryMb') ?? 512,
      maxCpuPercent: prefs.getDouble('maxCpuPercent') ?? 50.0,
      maxConcurrentAiTasks: prefs.getInt('maxConcurrentAiTasks') ?? 4,
      maxDatabaseSizeMb: prefs.getInt('maxDatabaseSizeMb') ?? 1024,
      adaptiveManagement: prefs.getBool('adaptiveManagement') ?? true,
      backgroundIntervalSecs: prefs.getInt('backgroundIntervalSecs') ?? 30,
      cacheSizeMb: prefs.getInt('cacheSizeMb') ?? 100,
    );

    notifyListeners();
  }

  /// Save settings to storage
  Future<void> _saveSettings() async {
    final prefs = await SharedPreferences.getInstance();

    await prefs.setInt('themeMode', _themeMode.index);
    await prefs.setBool('minimizeToTray', _minimizeToTray);
    await prefs.setBool('showNotifications', _showNotifications);
    await prefs.setBool('enableHotkeys', _enableHotkeys);
    await prefs.setBool('autoRefresh', _autoRefresh);
    await prefs.setInt('autoRefreshInterval', _autoRefreshInterval);
    await prefs.setString('defaultBrowser', _defaultBrowser);

    // Performance settings
    await prefs.setBool(
        'enablePerformanceMonitoring', _enablePerformanceMonitoring);
    await prefs.setInt('performanceHistoryHours', _performanceHistoryHours);

    // Resource config
    await prefs.setInt('maxMemoryMb', _resourceConfig.maxMemoryMb);
    await prefs.setDouble('maxCpuPercent', _resourceConfig.maxCpuPercent);
    await prefs.setInt(
        'maxConcurrentAiTasks', _resourceConfig.maxConcurrentAiTasks);
    await prefs.setInt('maxDatabaseSizeMb', _resourceConfig.maxDatabaseSizeMb);
    await prefs.setBool(
        'adaptiveManagement', _resourceConfig.adaptiveManagement);
    await prefs.setInt(
        'backgroundIntervalSecs', _resourceConfig.backgroundIntervalSecs);
    await prefs.setInt('cacheSizeMb', _resourceConfig.cacheSizeMb);
  }

  /// Set theme mode
  Future<void> setThemeMode(ThemeMode mode) async {
    _themeMode = mode;
    notifyListeners();
    await _saveSettings();
  }

  /// Set minimize to tray
  Future<void> setMinimizeToTray(bool value) async {
    _minimizeToTray = value;
    notifyListeners();
    await _saveSettings();
  }

  /// Set show notifications
  Future<void> setShowNotifications(bool value) async {
    _showNotifications = value;
    notifyListeners();
    await _saveSettings();
  }

  /// Set enable hotkeys
  Future<void> setEnableHotkeys(bool value) async {
    _enableHotkeys = value;
    notifyListeners();
    await _saveSettings();
  }

  /// Set auto refresh
  Future<void> setAutoRefresh(bool value) async {
    _autoRefresh = value;
    notifyListeners();
    await _saveSettings();
  }

  /// Set auto refresh interval
  Future<void> setAutoRefreshInterval(int seconds) async {
    _autoRefreshInterval = seconds;
    notifyListeners();
    await _saveSettings();
  }

  /// Set default browser
  Future<void> setDefaultBrowser(String browser) async {
    _defaultBrowser = browser;
    notifyListeners();
    await _saveSettings();
  }

  /// Set enable performance monitoring
  Future<void> setEnablePerformanceMonitoring(bool value) async {
    _enablePerformanceMonitoring = value;
    notifyListeners();
    await _saveSettings();
  }

  /// Set performance history hours
  Future<void> setPerformanceHistoryHours(int hours) async {
    _performanceHistoryHours = hours;
    notifyListeners();
    await _saveSettings();
  }

  /// Set resource config
  Future<void> setResourceConfig(ResourceConfig config) async {
    _resourceConfig = config;
    notifyListeners();
    await _saveSettings();
  }

  /// Update max memory limit
  Future<void> setMaxMemoryMb(int value) async {
    _resourceConfig = _resourceConfig.copyWith(maxMemoryMb: value);
    notifyListeners();
    await _saveSettings();
  }

  /// Update max CPU limit
  Future<void> setMaxCpuPercent(double value) async {
    _resourceConfig = _resourceConfig.copyWith(maxCpuPercent: value);
    notifyListeners();
    await _saveSettings();
  }

  /// Update adaptive management
  Future<void> setAdaptiveManagement(bool value) async {
    _resourceConfig = _resourceConfig.copyWith(adaptiveManagement: value);
    notifyListeners();
    await _saveSettings();
  }

  /// Update max concurrent AI tasks
  Future<void> setMaxConcurrentAiTasks(int value) async {
    _resourceConfig = _resourceConfig.copyWith(maxConcurrentAiTasks: value);
    notifyListeners();
    await _saveSettings();
  }

  /// Update cache size
  Future<void> setCacheSizeMb(int value) async {
    _resourceConfig = _resourceConfig.copyWith(cacheSizeMb: value);
    notifyListeners();
    await _saveSettings();
  }

  /// Reset to defaults
  Future<void> resetToDefaults() async {
    _themeMode = ThemeMode.system;
    _minimizeToTray = true;
    _showNotifications = true;
    _enableHotkeys = true;
    _autoRefresh = true;
    _autoRefreshInterval = 30;
    _defaultBrowser = 'chrome';
    _enablePerformanceMonitoring = true;
    _performanceHistoryHours = 24;
    _resourceConfig = ResourceConfig.defaults();

    notifyListeners();
    await _saveSettings();
  }
}
