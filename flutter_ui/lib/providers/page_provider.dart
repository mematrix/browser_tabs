import 'package:flutter/foundation.dart';
import '../models/page_info.dart';
import '../models/smart_group.dart';
import '../services/rust_bridge.dart';

/// Provider for managing page data (tabs, bookmarks, groups)
class PageProvider extends ChangeNotifier {
  final RustBridge _rustBridge;
  
  List<UnifiedPageInfo> _pages = [];
  List<SmartGroup> _groups = [];
  bool _isLoading = false;
  String? _error;
  int _connectedBrowserCount = 0;
  
  PageProvider(this._rustBridge) {
    _loadData();
  }
  
  // Getters
  List<UnifiedPageInfo> get pages => _pages;
  List<SmartGroup> get groups => _groups;
  bool get isLoading => _isLoading;
  String? get error => _error;
  int get connectedBrowserCount => _connectedBrowserCount;
  
  /// Get active tabs only
  List<UnifiedPageInfo> get activeTabs => 
      _pages.where((p) => p.sourceType == PageSourceType.activeTab).toList();
  
  /// Get bookmarks only
  List<UnifiedPageInfo> get bookmarks => 
      _pages.where((p) => p.sourceType == PageSourceType.bookmark).toList();
  
  /// Get history (closed tabs) only
  List<UnifiedPageInfo> get history => 
      _pages.where((p) => p.sourceType == PageSourceType.closedTab).toList();
  
  /// Get pages by browser type
  List<UnifiedPageInfo> getPagesByBrowser(BrowserType browser) =>
      _pages.where((p) => p.browserType == browser).toList();
  
  /// Get pages by group
  List<UnifiedPageInfo> getPagesByGroup(SmartGroup group) =>
      _pages.where((p) => group.pageIds.contains(p.id)).toList();
  
  /// Get tabs with pending changes
  List<UnifiedPageInfo> get tabsWithPendingChanges =>
      activeTabs.where((t) => t.hasPendingChanges).toList();
  
  /// Get tabs with bookmarks
  List<UnifiedPageInfo> get tabsWithBookmarks =>
      activeTabs.where((t) => t.hasBookmark).toList();
  
  /// Get page by ID
  UnifiedPageInfo? getPageById(String id) {
    try {
      return _pages.firstWhere((p) => p.id == id);
    } catch (_) {
      return null;
    }
  }
  
  /// Load all data
  Future<void> _loadData() async {
    _isLoading = true;
    _error = null;
    notifyListeners();
    
    try {
      _pages = await _rustBridge.getUnifiedPages();
      _groups = await _rustBridge.getSmartGroups();
      _connectedBrowserCount = await _rustBridge.getConnectedBrowserCount();
      _error = null;
    } catch (e) {
      _error = e.toString();
    } finally {
      _isLoading = false;
      notifyListeners();
    }
  }
  
  /// Refresh data
  Future<void> refresh() async {
    await _loadData();
  }
  
  /// Close a tab
  Future<void> closeTab(UnifiedPageInfo tab) async {
    if (tab.sourceType != PageSourceType.activeTab || tab.browserType == null) {
      return;
    }
    
    try {
      await _rustBridge.closeTab(tab.id, tab.browserType!);
      await refresh();
    } catch (e) {
      _error = e.toString();
      notifyListeners();
    }
  }
  
  /// Activate a tab
  Future<void> activateTab(UnifiedPageInfo tab) async {
    if (tab.sourceType != PageSourceType.activeTab || tab.browserType == null) {
      return;
    }
    
    try {
      await _rustBridge.activateTab(tab.id, tab.browserType!);
    } catch (e) {
      _error = e.toString();
      notifyListeners();
    }
  }
  
  /// Create bookmark from tab
  Future<void> createBookmarkFromTab(UnifiedPageInfo tab) async {
    if (tab.sourceType != PageSourceType.activeTab) {
      return;
    }
    
    try {
      await _rustBridge.createBookmarkFromTab(tab.id);
      await refresh();
    } catch (e) {
      _error = e.toString();
      notifyListeners();
    }
  }
  
  /// Get statistics
  PageStats get stats => PageStats(
    totalTabs: activeTabs.length,
    totalBookmarks: bookmarks.length,
    totalHistory: history.length,
    totalGroups: groups.length,
    connectedBrowsers: _connectedBrowserCount,
    tabsWithBookmarks: tabsWithBookmarks.length,
    pendingChanges: tabsWithPendingChanges.length,
  );
}

/// Page statistics
class PageStats {
  final int totalTabs;
  final int totalBookmarks;
  final int totalHistory;
  final int totalGroups;
  final int connectedBrowsers;
  final int tabsWithBookmarks;
  final int pendingChanges;
  
  PageStats({
    required this.totalTabs,
    required this.totalBookmarks,
    required this.totalHistory,
    required this.totalGroups,
    required this.connectedBrowsers,
    required this.tabsWithBookmarks,
    required this.pendingChanges,
  });
  
  int get totalPages => totalTabs + totalBookmarks + totalHistory;
}
