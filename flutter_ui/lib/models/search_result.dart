import 'page_info.dart';

/// Search result source type
enum SearchResultSource {
  activeTab,
  bookmark,
  history,
  archive,
  unifiedPage,
}

/// Search result item
class SearchResultItem {
  final String id;
  final String url;
  final String title;
  final SearchResultSource source;
  final BrowserType? browserType;
  final double relevanceScore;
  final String? snippet;
  final List<String> keywords;
  final DateTime? lastAccessed;

  SearchResultItem({
    required this.id,
    required this.url,
    required this.title,
    required this.source,
    this.browserType,
    required this.relevanceScore,
    this.snippet,
    required this.keywords,
    this.lastAccessed,
  });

  factory SearchResultItem.fromJson(Map<String, dynamic> json) {
    return SearchResultItem(
      id: json['id'] ?? '',
      url: json['url'] ?? '',
      title: json['title'] ?? '',
      source: _parseSource(json['source']),
      browserType: _parseBrowserType(json['browser_type']),
      relevanceScore: (json['relevance_score'] ?? 0.0).toDouble(),
      snippet: json['snippet'],
      keywords: List<String>.from(json['keywords'] ?? []),
      lastAccessed: json['last_accessed'] != null 
          ? DateTime.tryParse(json['last_accessed']) 
          : null,
    );
  }

  static SearchResultSource _parseSource(dynamic value) {
    if (value is String) {
      switch (value) {
        case 'ActiveTab': return SearchResultSource.activeTab;
        case 'Bookmark': return SearchResultSource.bookmark;
        case 'History': return SearchResultSource.history;
        case 'Archive': return SearchResultSource.archive;
        case 'UnifiedPage': return SearchResultSource.unifiedPage;
      }
    }
    return SearchResultSource.unifiedPage;
  }

  static BrowserType? _parseBrowserType(dynamic value) {
    if (value is String) {
      switch (value.toLowerCase()) {
        case 'chrome': return BrowserType.chrome;
        case 'firefox': return BrowserType.firefox;
        case 'edge': return BrowserType.edge;
        case 'safari': return BrowserType.safari;
      }
    }
    return null;
  }

  String get domain {
    try {
      final uri = Uri.parse(url);
      return uri.host;
    } catch (_) {
      return url;
    }
  }
}

/// Search results container
class SearchResults {
  final int totalCount;
  final List<SearchResultItem> items;
  final Duration searchTime;
  final Map<SearchResultSource, int> countBySource;

  SearchResults({
    required this.totalCount,
    required this.items,
    required this.searchTime,
    required this.countBySource,
  });

  factory SearchResults.fromJson(Map<String, dynamic> json) {
    final items = (json['items'] as List?)
        ?.map((e) => SearchResultItem.fromJson(e))
        .toList() ?? [];
    
    return SearchResults(
      totalCount: json['total_count'] ?? items.length,
      items: items,
      searchTime: Duration(milliseconds: json['search_time_ms'] ?? 0),
      countBySource: _parseCountBySource(json['count_by_source']),
    );
  }

  static Map<SearchResultSource, int> _parseCountBySource(dynamic value) {
    if (value is Map) {
      return value.map((k, v) => MapEntry(
        SearchResultItem._parseSource(k),
        v as int,
      ));
    }
    return {};
  }

  factory SearchResults.empty() {
    return SearchResults(
      totalCount: 0,
      items: [],
      searchTime: Duration.zero,
      countBySource: {},
    );
  }
}

/// Search suggestion
class SearchSuggestion {
  final String text;
  final String type;
  final double score;

  SearchSuggestion({
    required this.text,
    required this.type,
    required this.score,
  });

  factory SearchSuggestion.fromJson(Map<String, dynamic> json) {
    return SearchSuggestion(
      text: json['text'] ?? '',
      type: json['type'] ?? 'History',
      score: (json['score'] ?? 0.0).toDouble(),
    );
  }
}
