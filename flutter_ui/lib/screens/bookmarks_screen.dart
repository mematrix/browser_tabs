import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:url_launcher/url_launcher.dart';

import '../models/page_info.dart';
import '../providers/page_provider.dart';
import '../widgets/page_list_tile.dart';
import '../widgets/browser_filter_chips.dart';

/// Screen for viewing and managing bookmarks
class BookmarksScreen extends StatefulWidget {
  const BookmarksScreen({super.key});

  @override
  State<BookmarksScreen> createState() => _BookmarksScreenState();
}

class _BookmarksScreenState extends State<BookmarksScreen> {
  BrowserType? _selectedBrowser;
  String _searchQuery = '';
  String? _selectedCategory;
  BookmarkSortOrder _sortOrder = BookmarkSortOrder.recent;
  
  @override
  Widget build(BuildContext context) {
    final pageProvider = context.watch<PageProvider>();
    final bookmarks = _filterAndSortBookmarks(pageProvider.bookmarks);
    final categories = _getCategories(pageProvider.bookmarks);
    
    return Scaffold(
      appBar: AppBar(
        title: const Text('书签'),
        actions: [
          PopupMenuButton<BookmarkSortOrder>(
            icon: const Icon(Icons.sort),
            tooltip: '排序',
            onSelected: (order) {
              setState(() {
                _sortOrder = order;
              });
            },
            itemBuilder: (context) => [
              const PopupMenuItem(
                value: BookmarkSortOrder.recent,
                child: Text('最近访问'),
              ),
              const PopupMenuItem(
                value: BookmarkSortOrder.oldest,
                child: Text('最早添加'),
              ),
              const PopupMenuItem(
                value: BookmarkSortOrder.titleAsc,
                child: Text('标题 A-Z'),
              ),
              const PopupMenuItem(
                value: BookmarkSortOrder.titleDesc,
                child: Text('标题 Z-A'),
              ),
              const PopupMenuItem(
                value: BookmarkSortOrder.mostVisited,
                child: Text('访问最多'),
              ),
            ],
          ),
          IconButton(
            icon: const Icon(Icons.refresh),
            onPressed: pageProvider.isLoading ? null : pageProvider.refresh,
            tooltip: '刷新',
          ),
        ],
      ),
      body: Column(
        children: [
          // Search bar
          Padding(
            padding: const EdgeInsets.all(16),
            child: TextField(
              decoration: const InputDecoration(
                hintText: '搜索书签...',
                prefixIcon: Icon(Icons.search),
              ),
              onChanged: (value) {
                setState(() {
                  _searchQuery = value;
                });
              },
            ),
          ),
          
          // Filters
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                // Browser filter
                BrowserFilterChips(
                  selectedBrowser: _selectedBrowser,
                  onBrowserSelected: (browser) {
                    setState(() {
                      _selectedBrowser = browser;
                    });
                  },
                ),
                const SizedBox(height: 8),
                // Category filter
                if (categories.isNotEmpty)
                  SizedBox(
                    height: 36,
                    child: ListView(
                      scrollDirection: Axis.horizontal,
                      children: [
                        FilterChip(
                          label: const Text('全部'),
                          selected: _selectedCategory == null,
                          onSelected: (_) {
                            setState(() {
                              _selectedCategory = null;
                            });
                          },
                        ),
                        const SizedBox(width: 8),
                        ...categories.map((category) => Padding(
                          padding: const EdgeInsets.only(right: 8),
                          child: FilterChip(
                            label: Text(category),
                            selected: _selectedCategory == category,
                            onSelected: (_) {
                              setState(() {
                                _selectedCategory = _selectedCategory == category 
                                    ? null 
                                    : category;
                              });
                            },
                          ),
                        )),
                      ],
                    ),
                  ),
              ],
            ),
          ),
          const SizedBox(height: 8),
          
          // Bookmark count
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            child: Row(
              children: [
                Text(
                  '${bookmarks.length} 个书签',
                  style: Theme.of(context).textTheme.bodySmall,
                ),
              ],
            ),
          ),
          const SizedBox(height: 8),
          
          // Bookmark list
          Expanded(
            child: pageProvider.isLoading
                ? const Center(child: CircularProgressIndicator())
                : bookmarks.isEmpty
                    ? const Center(child: Text('暂无书签'))
                    : ListView.builder(
                        padding: const EdgeInsets.symmetric(horizontal: 16),
                        itemCount: bookmarks.length,
                        itemBuilder: (context, index) {
                          final bookmark = bookmarks[index];
                          return Card(
                            margin: const EdgeInsets.only(bottom: 8),
                            child: PageListTile(
                              page: bookmark,
                              onTap: () => _openBookmark(bookmark),
                              showCategory: true,
                              trailing: _buildBookmarkActions(bookmark),
                            ),
                          );
                        },
                      ),
          ),
        ],
      ),
      floatingActionButton: FloatingActionButton.extended(
        onPressed: () {
          // TODO: Show import bookmarks dialog
        },
        icon: const Icon(Icons.download),
        label: const Text('导入书签'),
      ),
    );
  }
  
  List<UnifiedPageInfo> _filterAndSortBookmarks(List<UnifiedPageInfo> bookmarks) {
    var filtered = bookmarks;
    
    // Filter by browser
    if (_selectedBrowser != null) {
      filtered = filtered.where((b) => b.browserType == _selectedBrowser).toList();
    }
    
    // Filter by category
    if (_selectedCategory != null) {
      filtered = filtered.where((b) => b.category == _selectedCategory).toList();
    }
    
    // Filter by search query
    if (_searchQuery.isNotEmpty) {
      final query = _searchQuery.toLowerCase();
      filtered = filtered.where((b) =>
        b.title.toLowerCase().contains(query) ||
        b.url.toLowerCase().contains(query) ||
        b.keywords.any((k) => k.toLowerCase().contains(query))
      ).toList();
    }
    
    // Sort
    switch (_sortOrder) {
      case BookmarkSortOrder.recent:
        filtered.sort((a, b) => b.lastAccessed.compareTo(a.lastAccessed));
        break;
      case BookmarkSortOrder.oldest:
        filtered.sort((a, b) => a.createdAt.compareTo(b.createdAt));
        break;
      case BookmarkSortOrder.titleAsc:
        filtered.sort((a, b) => a.title.compareTo(b.title));
        break;
      case BookmarkSortOrder.titleDesc:
        filtered.sort((a, b) => b.title.compareTo(a.title));
        break;
      case BookmarkSortOrder.mostVisited:
        filtered.sort((a, b) => b.accessCount.compareTo(a.accessCount));
        break;
    }
    
    return filtered;
  }
  
  List<String> _getCategories(List<UnifiedPageInfo> bookmarks) {
    final categories = <String>{};
    for (final bookmark in bookmarks) {
      if (bookmark.category != null) {
        categories.add(bookmark.category!);
      }
    }
    return categories.toList()..sort();
  }
  
  Future<void> _openBookmark(UnifiedPageInfo bookmark) async {
    final uri = Uri.tryParse(bookmark.url);
    if (uri != null && await canLaunchUrl(uri)) {
      await launchUrl(uri);
    }
  }
  
  Widget _buildBookmarkActions(UnifiedPageInfo bookmark) {
    return PopupMenuButton<String>(
      icon: const Icon(Icons.more_vert),
      onSelected: (action) {
        switch (action) {
          case 'open':
            _openBookmark(bookmark);
            break;
          case 'copy':
            // TODO: Copy URL to clipboard
            break;
          case 'edit':
            // TODO: Show edit dialog
            break;
          case 'delete':
            // TODO: Show delete confirmation
            break;
        }
      },
      itemBuilder: (context) => [
        const PopupMenuItem(
          value: 'open',
          child: ListTile(
            leading: Icon(Icons.open_in_new),
            title: Text('打开'),
            contentPadding: EdgeInsets.zero,
          ),
        ),
        const PopupMenuItem(
          value: 'copy',
          child: ListTile(
            leading: Icon(Icons.copy),
            title: Text('复制链接'),
            contentPadding: EdgeInsets.zero,
          ),
        ),
        const PopupMenuItem(
          value: 'edit',
          child: ListTile(
            leading: Icon(Icons.edit),
            title: Text('编辑'),
            contentPadding: EdgeInsets.zero,
          ),
        ),
        const PopupMenuItem(
          value: 'delete',
          child: ListTile(
            leading: Icon(Icons.delete, color: Colors.red),
            title: Text('删除', style: TextStyle(color: Colors.red)),
            contentPadding: EdgeInsets.zero,
          ),
        ),
      ],
    );
  }
}

enum BookmarkSortOrder {
  recent,
  oldest,
  titleAsc,
  titleDesc,
  mostVisited,
}
