import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:url_launcher/url_launcher.dart';

import '../models/page_info.dart';
import '../models/search_result.dart';
import '../providers/search_provider.dart';
import '../widgets/search_result_tile.dart';

/// Screen for unified search across all data sources
class SearchScreen extends StatefulWidget {
  const SearchScreen({super.key});

  @override
  State<SearchScreen> createState() => _SearchScreenState();
}

class _SearchScreenState extends State<SearchScreen> {
  final _searchController = TextEditingController();
  final _focusNode = FocusNode();
  
  @override
  void initState() {
    super.initState();
    _focusNode.requestFocus();
  }
  
  @override
  void dispose() {
    _searchController.dispose();
    _focusNode.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final searchProvider = context.watch<SearchProvider>();
    
    return Scaffold(
      appBar: AppBar(
        title: const Text('搜索'),
      ),
      body: Column(
        children: [
          // Search input
          Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              children: [
                TextField(
                  controller: _searchController,
                  focusNode: _focusNode,
                  decoration: InputDecoration(
                    hintText: '搜索标签页、书签、历史记录...',
                    prefixIcon: const Icon(Icons.search),
                    suffixIcon: _searchController.text.isNotEmpty
                        ? IconButton(
                            icon: const Icon(Icons.clear),
                            onPressed: () {
                              _searchController.clear();
                              searchProvider.clearSearch();
                            },
                          )
                        : null,
                  ),
                  onChanged: (value) {
                    searchProvider.updateQuery(value);
                  },
                  onSubmitted: (_) {
                    searchProvider.search();
                  },
                ),
                const SizedBox(height: 12),
                
                // Source filters
                _buildSourceFilters(searchProvider),
                const SizedBox(height: 8),
                
                // Sort options
                _buildSortOptions(searchProvider),
              ],
            ),
          ),
          
          // Suggestions or results
          Expanded(
            child: searchProvider.isSearching
                ? const Center(child: CircularProgressIndicator())
                : searchProvider.results.items.isEmpty
                    ? _buildSuggestionsOrEmpty(searchProvider)
                    : _buildSearchResults(searchProvider),
          ),
        ],
      ),
    );
  }
  
  Widget _buildSourceFilters(SearchProvider provider) {
    return SingleChildScrollView(
      scrollDirection: Axis.horizontal,
      child: Row(
        children: [
          FilterChip(
            label: const Text('标签页'),
            selected: provider.selectedSources.contains(SearchResultSource.activeTab),
            onSelected: (_) => provider.toggleSource(SearchResultSource.activeTab),
            avatar: const Icon(Icons.tab, size: 16),
          ),
          const SizedBox(width: 8),
          FilterChip(
            label: const Text('书签'),
            selected: provider.selectedSources.contains(SearchResultSource.bookmark),
            onSelected: (_) => provider.toggleSource(SearchResultSource.bookmark),
            avatar: const Icon(Icons.bookmark, size: 16),
          ),
          const SizedBox(width: 8),
          FilterChip(
            label: const Text('历史'),
            selected: provider.selectedSources.contains(SearchResultSource.history),
            onSelected: (_) => provider.toggleSource(SearchResultSource.history),
            avatar: const Icon(Icons.history, size: 16),
          ),
          const SizedBox(width: 8),
          FilterChip(
            label: const Text('存档'),
            selected: provider.selectedSources.contains(SearchResultSource.archive),
            onSelected: (_) => provider.toggleSource(SearchResultSource.archive),
            avatar: const Icon(Icons.archive, size: 16),
          ),
        ],
      ),
    );
  }
  
  Widget _buildSortOptions(SearchProvider provider) {
    return Row(
      children: [
        const Text('排序: '),
        DropdownButton<SearchSortOrder>(
          value: provider.sortOrder,
          underline: const SizedBox(),
          items: SearchSortOrder.values.map((order) => DropdownMenuItem(
            value: order,
            child: Text(order.displayName),
          )).toList(),
          onChanged: (order) {
            if (order != null) {
              provider.setSortOrder(order);
            }
          },
        ),
      ],
    );
  }
  
  Widget _buildSuggestionsOrEmpty(SearchProvider provider) {
    if (provider.query.isEmpty) {
      // Show search history
      if (provider.searchHistory.isEmpty) {
        return Center(
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              Icon(
                Icons.search,
                size: 64,
                color: Theme.of(context).colorScheme.outline,
              ),
              const SizedBox(height: 16),
              Text(
                '输入关键词开始搜索',
                style: Theme.of(context).textTheme.bodyLarge?.copyWith(
                  color: Theme.of(context).colorScheme.outline,
                ),
              ),
            ],
          ),
        );
      }
      
      return Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Text(
                  '搜索历史',
                  style: Theme.of(context).textTheme.titleMedium,
                ),
                TextButton(
                  onPressed: provider.clearHistory,
                  child: const Text('清除'),
                ),
              ],
            ),
          ),
          Expanded(
            child: ListView.builder(
              itemCount: provider.searchHistory.length,
              itemBuilder: (context, index) {
                final query = provider.searchHistory[index];
                return ListTile(
                  leading: const Icon(Icons.history),
                  title: Text(query),
                  trailing: IconButton(
                    icon: const Icon(Icons.close),
                    onPressed: () => provider.removeFromHistory(query),
                  ),
                  onTap: () {
                    _searchController.text = query;
                    provider.updateQuery(query);
                    provider.search();
                  },
                );
              },
            ),
          ),
        ],
      );
    }
    
    // Show suggestions
    if (provider.suggestions.isNotEmpty) {
      return ListView.builder(
        itemCount: provider.suggestions.length,
        itemBuilder: (context, index) {
          final suggestion = provider.suggestions[index];
          return ListTile(
            leading: Icon(_getSuggestionIcon(suggestion.type)),
            title: Text(suggestion.text),
            onTap: () {
              _searchController.text = suggestion.text;
              provider.updateQuery(suggestion.text);
              provider.search();
            },
          );
        },
      );
    }
    
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(
            Icons.search_off,
            size: 64,
            color: Theme.of(context).colorScheme.outline,
          ),
          const SizedBox(height: 16),
          Text(
            '未找到结果',
            style: Theme.of(context).textTheme.bodyLarge?.copyWith(
              color: Theme.of(context).colorScheme.outline,
            ),
          ),
        ],
      ),
    );
  }
  
  Widget _buildSearchResults(SearchProvider provider) {
    final results = provider.results;
    
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        // Results summary
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16),
          child: Row(
            children: [
              Text(
                '找到 ${results.totalCount} 个结果',
                style: Theme.of(context).textTheme.bodySmall,
              ),
              const SizedBox(width: 8),
              Text(
                '(${results.searchTime.inMilliseconds}ms)',
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                  color: Theme.of(context).colorScheme.outline,
                ),
              ),
            ],
          ),
        ),
        
        // Source breakdown
        if (results.countBySource.isNotEmpty)
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
            child: Wrap(
              spacing: 8,
              children: results.countBySource.entries.map((entry) => Chip(
                label: Text('${_getSourceName(entry.key)}: ${entry.value}'),
                visualDensity: VisualDensity.compact,
              )).toList(),
            ),
          ),
        
        // Results list
        Expanded(
          child: ListView.builder(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            itemCount: results.items.length,
            itemBuilder: (context, index) {
              final item = results.items[index];
              return Card(
                margin: const EdgeInsets.only(bottom: 8),
                child: SearchResultTile(
                  result: item,
                  onTap: () => _openResult(item),
                ),
              );
            },
          ),
        ),
      ],
    );
  }
  
  IconData _getSuggestionIcon(String type) {
    switch (type) {
      case 'History':
        return Icons.history;
      case 'Title':
        return Icons.title;
      case 'Keyword':
        return Icons.label;
      case 'Url':
        return Icons.link;
      default:
        return Icons.search;
    }
  }
  
  String _getSourceName(SearchResultSource source) {
    switch (source) {
      case SearchResultSource.activeTab:
        return '标签页';
      case SearchResultSource.bookmark:
        return '书签';
      case SearchResultSource.history:
        return '历史';
      case SearchResultSource.archive:
        return '存档';
      case SearchResultSource.unifiedPage:
        return '页面';
    }
  }
  
  Future<void> _openResult(SearchResultItem item) async {
    final uri = Uri.tryParse(item.url);
    if (uri != null && await canLaunchUrl(uri)) {
      await launchUrl(uri);
    }
  }
}
