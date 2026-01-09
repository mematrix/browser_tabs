import 'package:flutter/foundation.dart';
import '../models/page_info.dart';
import '../models/search_result.dart';
import '../services/rust_bridge.dart';

/// Provider for managing search functionality
class SearchProvider extends ChangeNotifier {
  final RustBridge _rustBridge;
  
  String _query = '';
  SearchResults _results = SearchResults.empty();
  List<SearchSuggestion> _suggestions = [];
  bool _isSearching = false;
  String? _error;
  
  // Filter options
  Set<SearchResultSource> _selectedSources = SearchResultSource.values.toSet();
  BrowserType? _selectedBrowser;
  SearchSortOrder _sortOrder = SearchSortOrder.relevance;
  
  // Search history
  final List<String> _searchHistory = [];
  static const int _maxHistoryItems = 20;
  
  SearchProvider(this._rustBridge);
  
  // Getters
  String get query => _query;
  SearchResults get results => _results;
  List<SearchSuggestion> get suggestions => _suggestions;
  bool get isSearching => _isSearching;
  String? get error => _error;
  Set<SearchResultSource> get selectedSources => _selectedSources;
  BrowserType? get selectedBrowser => _selectedBrowser;
  SearchSortOrder get sortOrder => _sortOrder;
  List<String> get searchHistory => List.unmodifiable(_searchHistory);
  
  /// Update query and fetch suggestions
  Future<void> updateQuery(String query) async {
    _query = query;
    notifyListeners();
    
    if (query.isEmpty) {
      _suggestions = [];
      notifyListeners();
      return;
    }
    
    try {
      _suggestions = await _rustBridge.getSuggestions(query);
      notifyListeners();
    } catch (e) {
      // Ignore suggestion errors
    }
  }
  
  /// Execute search
  Future<void> search() async {
    if (_query.isEmpty) {
      _results = SearchResults.empty();
      notifyListeners();
      return;
    }
    
    _isSearching = true;
    _error = null;
    notifyListeners();
    
    try {
      _results = await _rustBridge.search(
        _query,
        sources: _selectedSources.toList(),
        browserType: _selectedBrowser,
      );
      
      // Sort results
      _sortResults();
      
      // Add to history
      _addToHistory(_query);
      
      _error = null;
    } catch (e) {
      _error = e.toString();
      _results = SearchResults.empty();
    } finally {
      _isSearching = false;
      notifyListeners();
    }
  }
  
  /// Set source filter
  void setSourceFilter(Set<SearchResultSource> sources) {
    _selectedSources = sources;
    notifyListeners();
  }
  
  /// Toggle source filter
  void toggleSource(SearchResultSource source) {
    if (_selectedSources.contains(source)) {
      _selectedSources.remove(source);
    } else {
      _selectedSources.add(source);
    }
    notifyListeners();
  }
  
  /// Set browser filter
  void setBrowserFilter(BrowserType? browser) {
    _selectedBrowser = browser;
    notifyListeners();
  }
  
  /// Set sort order
  void setSortOrder(SearchSortOrder order) {
    _sortOrder = order;
    _sortResults();
    notifyListeners();
  }
  
  /// Clear search
  void clearSearch() {
    _query = '';
    _results = SearchResults.empty();
    _suggestions = [];
    _error = null;
    notifyListeners();
  }
  
  /// Clear history
  void clearHistory() {
    _searchHistory.clear();
    notifyListeners();
  }
  
  /// Remove item from history
  void removeFromHistory(String query) {
    _searchHistory.remove(query);
    notifyListeners();
  }
  
  void _sortResults() {
    final items = List<SearchResultItem>.from(_results.items);
    
    switch (_sortOrder) {
      case SearchSortOrder.relevance:
        items.sort((a, b) => b.relevanceScore.compareTo(a.relevanceScore));
        break;
      case SearchSortOrder.newestFirst:
        items.sort((a, b) {
          final aTime = a.lastAccessed ?? DateTime(1970);
          final bTime = b.lastAccessed ?? DateTime(1970);
          return bTime.compareTo(aTime);
        });
        break;
      case SearchSortOrder.oldestFirst:
        items.sort((a, b) {
          final aTime = a.lastAccessed ?? DateTime(1970);
          final bTime = b.lastAccessed ?? DateTime(1970);
          return aTime.compareTo(bTime);
        });
        break;
      case SearchSortOrder.titleAsc:
        items.sort((a, b) => a.title.compareTo(b.title));
        break;
      case SearchSortOrder.titleDesc:
        items.sort((a, b) => b.title.compareTo(a.title));
        break;
    }
    
    _results = SearchResults(
      totalCount: _results.totalCount,
      items: items,
      searchTime: _results.searchTime,
      countBySource: _results.countBySource,
    );
  }
  
  void _addToHistory(String query) {
    _searchHistory.remove(query);
    _searchHistory.insert(0, query);
    if (_searchHistory.length > _maxHistoryItems) {
      _searchHistory.removeLast();
    }
  }
}

/// Search sort order options
enum SearchSortOrder {
  relevance,
  newestFirst,
  oldestFirst,
  titleAsc,
  titleDesc,
}

extension SearchSortOrderExtension on SearchSortOrder {
  String get displayName {
    switch (this) {
      case SearchSortOrder.relevance:
        return '相关性';
      case SearchSortOrder.newestFirst:
        return '最新优先';
      case SearchSortOrder.oldestFirst:
        return '最早优先';
      case SearchSortOrder.titleAsc:
        return '标题 A-Z';
      case SearchSortOrder.titleDesc:
        return '标题 Z-A';
    }
  }
}
