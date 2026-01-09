import 'dart:convert';
import 'dart:ffi';
import 'dart:io';

import '../models/page_info.dart';
import '../models/smart_group.dart';
import '../models/search_result.dart';

/// Bridge to Rust core library via FFI
class RustBridge {
  bool _initialized = false;
  
  // In a real implementation, this would load the Rust dynamic library
  // and call FFI functions. For now, we provide mock data for UI development.
  
  Future<void> initialize() async {
    // TODO: Load Rust dynamic library
    // final dylib = Platform.isWindows
    //     ? DynamicLibrary.open('web_page_manager_core.dll')
    //     : Platform.isMacOS
    //         ? DynamicLibrary.open('libweb_page_manager_core.dylib')
    //         : DynamicLibrary.open('libweb_page_manager_core.so');
    
    _initialized = true;
  }
  
  bool get isInitialized => _initialized;
  
  /// Get all unified pages (tabs + bookmarks)
  Future<List<UnifiedPageInfo>> getUnifiedPages() async {
    // TODO: Call Rust FFI function
    // For now, return mock data for UI development
    return _getMockPages();
  }
  
  /// Get all smart groups
  Future<List<SmartGroup>> getSmartGroups() async {
    // TODO: Call Rust FFI function
    return _getMockGroups();
  }
  
  /// Get active tabs from all browsers
  Future<List<UnifiedPageInfo>> getActiveTabs() async {
    final pages = await getUnifiedPages();
    return pages.where((p) => p.sourceType == PageSourceType.activeTab).toList();
  }
  
  /// Get all bookmarks
  Future<List<UnifiedPageInfo>> getBookmarks() async {
    final pages = await getUnifiedPages();
    return pages.where((p) => p.sourceType == PageSourceType.bookmark).toList();
  }
  
  /// Get tab history
  Future<List<UnifiedPageInfo>> getHistory() async {
    final pages = await getUnifiedPages();
    return pages.where((p) => p.sourceType == PageSourceType.closedTab).toList();
  }
  
  /// Search across all data sources
  Future<SearchResults> search(String query, {
    List<SearchResultSource>? sources,
    BrowserType? browserType,
    int limit = 50,
    int offset = 0,
  }) async {
    // TODO: Call Rust FFI function
    if (query.isEmpty) {
      return SearchResults.empty();
    }
    
    final pages = await getUnifiedPages();
    final queryLower = query.toLowerCase();
    
    final matchingPages = pages.where((p) =>
      p.title.toLowerCase().contains(queryLower) ||
      p.url.toLowerCase().contains(queryLower) ||
      p.keywords.any((k) => k.toLowerCase().contains(queryLower))
    ).toList();
    
    final items = matchingPages.map((p) => SearchResultItem(
      id: p.id,
      url: p.url,
      title: p.title,
      source: _pageSourceToSearchSource(p.sourceType),
      browserType: p.browserType,
      relevanceScore: _calculateRelevance(p, queryLower),
      snippet: p.contentSummary?.summaryText,
      keywords: p.keywords,
      lastAccessed: p.lastAccessed,
    )).toList();
    
    items.sort((a, b) => b.relevanceScore.compareTo(a.relevanceScore));
    
    return SearchResults(
      totalCount: items.length,
      items: items.skip(offset).take(limit).toList(),
      searchTime: const Duration(milliseconds: 15),
      countBySource: _countBySource(items),
    );
  }
  
  /// Get search suggestions
  Future<List<SearchSuggestion>> getSuggestions(String query) async {
    if (query.isEmpty) return [];
    
    // TODO: Call Rust FFI function
    return [
      SearchSuggestion(text: query, type: 'History', score: 1.0),
    ];
  }
  
  /// Close a tab
  Future<void> closeTab(String tabId, BrowserType browser) async {
    // TODO: Call Rust FFI function
  }
  
  /// Activate a tab
  Future<void> activateTab(String tabId, BrowserType browser) async {
    // TODO: Call Rust FFI function
  }
  
  /// Create a new tab
  Future<String> createTab(String url, BrowserType browser) async {
    // TODO: Call Rust FFI function
    return 'new-tab-id';
  }
  
  /// Create bookmark from tab
  Future<void> createBookmarkFromTab(String tabId) async {
    // TODO: Call Rust FFI function
  }
  
  /// Get connected browser count
  Future<int> getConnectedBrowserCount() async {
    // TODO: Call Rust FFI function
    return 2;
  }
  
  SearchResultSource _pageSourceToSearchSource(PageSourceType source) {
    switch (source) {
      case PageSourceType.activeTab:
        return SearchResultSource.activeTab;
      case PageSourceType.bookmark:
        return SearchResultSource.bookmark;
      case PageSourceType.closedTab:
        return SearchResultSource.history;
      case PageSourceType.archivedContent:
        return SearchResultSource.archive;
    }
  }
  
  double _calculateRelevance(UnifiedPageInfo page, String query) {
    double score = 0.0;
    if (page.title.toLowerCase().contains(query)) score += 0.5;
    if (page.url.toLowerCase().contains(query)) score += 0.3;
    if (page.keywords.any((k) => k.toLowerCase().contains(query))) score += 0.2;
    return score;
  }
  
  Map<SearchResultSource, int> _countBySource(List<SearchResultItem> items) {
    final counts = <SearchResultSource, int>{};
    for (final item in items) {
      counts[item.source] = (counts[item.source] ?? 0) + 1;
    }
    return counts;
  }
  
  List<UnifiedPageInfo> _getMockPages() {
    final now = DateTime.now();
    return [
      UnifiedPageInfo(
        id: '1',
        url: 'https://flutter.dev',
        title: 'Flutter - Build apps for any screen',
        faviconUrl: 'https://flutter.dev/favicon.ico',
        keywords: ['flutter', 'dart', 'mobile', 'web'],
        category: 'Development',
        sourceType: PageSourceType.activeTab,
        browserType: BrowserType.chrome,
        createdAt: now.subtract(const Duration(hours: 2)),
        lastAccessed: now,
        accessCount: 5,
        hasBookmark: true,
      ),
      UnifiedPageInfo(
        id: '2',
        url: 'https://rust-lang.org',
        title: 'Rust Programming Language',
        faviconUrl: 'https://rust-lang.org/favicon.ico',
        keywords: ['rust', 'programming', 'systems'],
        category: 'Development',
        sourceType: PageSourceType.activeTab,
        browserType: BrowserType.chrome,
        createdAt: now.subtract(const Duration(hours: 1)),
        lastAccessed: now,
        accessCount: 3,
      ),
      UnifiedPageInfo(
        id: '3',
        url: 'https://github.com',
        title: 'GitHub: Let\'s build from here',
        faviconUrl: 'https://github.com/favicon.ico',
        keywords: ['github', 'git', 'code', 'repository'],
        category: 'Development',
        sourceType: PageSourceType.bookmark,
        browserType: BrowserType.firefox,
        createdAt: now.subtract(const Duration(days: 30)),
        lastAccessed: now.subtract(const Duration(hours: 5)),
        accessCount: 100,
      ),
      UnifiedPageInfo(
        id: '4',
        url: 'https://news.ycombinator.com',
        title: 'Hacker News',
        faviconUrl: 'https://news.ycombinator.com/favicon.ico',
        keywords: ['news', 'tech', 'startup'],
        category: 'News',
        sourceType: PageSourceType.closedTab,
        browserType: BrowserType.edge,
        createdAt: now.subtract(const Duration(hours: 3)),
        lastAccessed: now.subtract(const Duration(minutes: 30)),
        accessCount: 15,
      ),
    ];
  }
  
  List<SmartGroup> _getMockGroups() {
    final now = DateTime.now();
    return [
      SmartGroup(
        id: 'g1',
        name: 'Development',
        description: 'Programming and development resources',
        groupType: GroupType.topic,
        pageIds: ['1', '2', '3'],
        createdAt: now.subtract(const Duration(days: 7)),
        autoGenerated: true,
        similarityThreshold: 0.8,
      ),
      SmartGroup(
        id: 'g2',
        name: 'News & Media',
        description: 'News and media websites',
        groupType: GroupType.contentType,
        pageIds: ['4'],
        createdAt: now.subtract(const Duration(days: 3)),
        autoGenerated: true,
        similarityThreshold: 0.7,
      ),
    ];
  }
}
