import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../models/page_info.dart';
import '../providers/page_provider.dart';
import '../widgets/page_list_tile.dart';
import '../widgets/browser_filter_chips.dart';

/// Screen for viewing and managing active tabs
class TabsScreen extends StatefulWidget {
  const TabsScreen({super.key});

  @override
  State<TabsScreen> createState() => _TabsScreenState();
}

class _TabsScreenState extends State<TabsScreen> {
  BrowserType? _selectedBrowser;
  String _searchQuery = '';
  TabViewMode _viewMode = TabViewMode.list;
  
  @override
  Widget build(BuildContext context) {
    final pageProvider = context.watch<PageProvider>();
    final tabs = _filterTabs(pageProvider.activeTabs);
    
    return Scaffold(
      appBar: AppBar(
        title: const Text('标签页'),
        actions: [
          IconButton(
            icon: Icon(_viewMode == TabViewMode.list 
                ? Icons.grid_view 
                : Icons.list),
            onPressed: () {
              setState(() {
                _viewMode = _viewMode == TabViewMode.list 
                    ? TabViewMode.grid 
                    : TabViewMode.list;
              });
            },
            tooltip: _viewMode == TabViewMode.list ? '网格视图' : '列表视图',
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
          // Search and filter bar
          Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              children: [
                TextField(
                  decoration: const InputDecoration(
                    hintText: '搜索标签页...',
                    prefixIcon: Icon(Icons.search),
                  ),
                  onChanged: (value) {
                    setState(() {
                      _searchQuery = value;
                    });
                  },
                ),
                const SizedBox(height: 12),
                BrowserFilterChips(
                  selectedBrowser: _selectedBrowser,
                  onBrowserSelected: (browser) {
                    setState(() {
                      _selectedBrowser = browser;
                    });
                  },
                ),
              ],
            ),
          ),
          
          // Tab count
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Text(
                  '${tabs.length} 个标签页',
                  style: Theme.of(context).textTheme.bodySmall,
                ),
                if (pageProvider.tabsWithPendingChanges.isNotEmpty)
                  Chip(
                    label: Text('${pageProvider.tabsWithPendingChanges.length} 个待同步'),
                    backgroundColor: Colors.orange.shade100,
                  ),
              ],
            ),
          ),
          const SizedBox(height: 8),
          
          // Tab list
          Expanded(
            child: pageProvider.isLoading
                ? const Center(child: CircularProgressIndicator())
                : tabs.isEmpty
                    ? const Center(child: Text('暂无标签页'))
                    : _viewMode == TabViewMode.list
                        ? _buildListView(tabs, pageProvider)
                        : _buildGridView(tabs, pageProvider),
          ),
        ],
      ),
      floatingActionButton: FloatingActionButton.extended(
        onPressed: () {
          // TODO: Show close all tabs dialog
        },
        icon: const Icon(Icons.close_fullscreen),
        label: const Text('批量操作'),
      ),
    );
  }
  
  List<UnifiedPageInfo> _filterTabs(List<UnifiedPageInfo> tabs) {
    var filtered = tabs;
    
    if (_selectedBrowser != null) {
      filtered = filtered.where((t) => t.browserType == _selectedBrowser).toList();
    }
    
    if (_searchQuery.isNotEmpty) {
      final query = _searchQuery.toLowerCase();
      filtered = filtered.where((t) =>
        t.title.toLowerCase().contains(query) ||
        t.url.toLowerCase().contains(query)
      ).toList();
    }
    
    return filtered;
  }
  
  Widget _buildListView(List<UnifiedPageInfo> tabs, PageProvider provider) {
    return ListView.builder(
      padding: const EdgeInsets.symmetric(horizontal: 16),
      itemCount: tabs.length,
      itemBuilder: (context, index) {
        final tab = tabs[index];
        return Card(
          margin: const EdgeInsets.only(bottom: 8),
          child: PageListTile(
            page: tab,
            onTap: () => provider.activateTab(tab),
            onClose: () => provider.closeTab(tab),
            onBookmark: tab.hasBookmark ? null : () => provider.createBookmarkFromTab(tab),
          ),
        );
      },
    );
  }
  
  Widget _buildGridView(List<UnifiedPageInfo> tabs, PageProvider provider) {
    return GridView.builder(
      padding: const EdgeInsets.all(16),
      gridDelegate: const SliverGridDelegateWithFixedCrossAxisCount(
        crossAxisCount: 3,
        childAspectRatio: 1.5,
        crossAxisSpacing: 12,
        mainAxisSpacing: 12,
      ),
      itemCount: tabs.length,
      itemBuilder: (context, index) {
        final tab = tabs[index];
        return _buildGridCard(tab, provider);
      },
    );
  }
  
  Widget _buildGridCard(UnifiedPageInfo tab, PageProvider provider) {
    return Card(
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: () => provider.activateTab(tab),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Header with favicon and browser icon
            Container(
              padding: const EdgeInsets.all(8),
              color: Theme.of(context).colorScheme.surfaceContainerHighest,
              child: Row(
                children: [
                  if (tab.faviconUrl != null)
                    Image.network(
                      tab.faviconUrl!,
                      width: 16,
                      height: 16,
                      errorBuilder: (_, __, ___) => const Icon(Icons.web, size: 16),
                    )
                  else
                    const Icon(Icons.web, size: 16),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      tab.domain,
                      style: Theme.of(context).textTheme.bodySmall,
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                  _buildBrowserIcon(tab.browserType),
                ],
              ),
            ),
            // Title
            Expanded(
              child: Padding(
                padding: const EdgeInsets.all(8),
                child: Text(
                  tab.title,
                  style: Theme.of(context).textTheme.bodyMedium,
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
            ),
            // Actions
            Padding(
              padding: const EdgeInsets.all(4),
              child: Row(
                mainAxisAlignment: MainAxisAlignment.end,
                children: [
                  if (!tab.hasBookmark)
                    IconButton(
                      icon: const Icon(Icons.bookmark_add_outlined, size: 18),
                      onPressed: () => provider.createBookmarkFromTab(tab),
                      tooltip: '添加书签',
                      visualDensity: VisualDensity.compact,
                    ),
                  IconButton(
                    icon: const Icon(Icons.close, size: 18),
                    onPressed: () => provider.closeTab(tab),
                    tooltip: '关闭',
                    visualDensity: VisualDensity.compact,
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
  
  Widget _buildBrowserIcon(BrowserType? browser) {
    IconData icon;
    Color color;
    
    switch (browser) {
      case BrowserType.chrome:
        icon = Icons.circle;
        color = Colors.blue;
        break;
      case BrowserType.firefox:
        icon = Icons.circle;
        color = Colors.orange;
        break;
      case BrowserType.edge:
        icon = Icons.circle;
        color = Colors.teal;
        break;
      case BrowserType.safari:
        icon = Icons.circle;
        color = Colors.blue.shade300;
        break;
      default:
        icon = Icons.circle;
        color = Colors.grey;
    }
    
    return Icon(icon, size: 12, color: color);
  }
}

enum TabViewMode { list, grid }
