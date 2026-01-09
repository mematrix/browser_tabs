import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:intl/intl.dart';

import '../models/page_info.dart';
import '../providers/page_provider.dart';
import '../widgets/page_list_tile.dart';
import '../widgets/browser_filter_chips.dart';

/// Screen for viewing and managing tab history
class HistoryScreen extends StatefulWidget {
  const HistoryScreen({super.key});

  @override
  State<HistoryScreen> createState() => _HistoryScreenState();
}

class _HistoryScreenState extends State<HistoryScreen> {
  BrowserType? _selectedBrowser;
  String _searchQuery = '';
  DateTimeRange? _dateRange;
  
  @override
  Widget build(BuildContext context) {
    final pageProvider = context.watch<PageProvider>();
    final history = _filterHistory(pageProvider.history);
    final groupedHistory = _groupByDate(history);
    
    return Scaffold(
      appBar: AppBar(
        title: const Text('历史记录'),
        actions: [
          IconButton(
            icon: const Icon(Icons.date_range),
            onPressed: () => _selectDateRange(context),
            tooltip: '选择日期范围',
          ),
          IconButton(
            icon: const Icon(Icons.delete_sweep),
            onPressed: history.isEmpty ? null : () => _showClearHistoryDialog(context),
            tooltip: '清除历史',
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
          // Search and filter
          Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              children: [
                TextField(
                  decoration: const InputDecoration(
                    hintText: '搜索历史记录...',
                    prefixIcon: Icon(Icons.search),
                  ),
                  onChanged: (value) {
                    setState(() {
                      _searchQuery = value;
                    });
                  },
                ),
                const SizedBox(height: 12),
                Row(
                  children: [
                    Expanded(
                      child: BrowserFilterChips(
                        selectedBrowser: _selectedBrowser,
                        onBrowserSelected: (browser) {
                          setState(() {
                            _selectedBrowser = browser;
                          });
                        },
                      ),
                    ),
                    if (_dateRange != null)
                      Chip(
                        label: Text(_formatDateRange(_dateRange!)),
                        onDeleted: () {
                          setState(() {
                            _dateRange = null;
                          });
                        },
                      ),
                  ],
                ),
              ],
            ),
          ),
          
          // History count
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            child: Row(
              children: [
                Text(
                  '${history.length} 条记录',
                  style: Theme.of(context).textTheme.bodySmall,
                ),
              ],
            ),
          ),
          const SizedBox(height: 8),
          
          // History list grouped by date
          Expanded(
            child: pageProvider.isLoading
                ? const Center(child: CircularProgressIndicator())
                : history.isEmpty
                    ? _buildEmptyState()
                    : ListView.builder(
                        padding: const EdgeInsets.symmetric(horizontal: 16),
                        itemCount: groupedHistory.length,
                        itemBuilder: (context, index) {
                          final entry = groupedHistory.entries.elementAt(index);
                          return _buildDateGroup(entry.key, entry.value);
                        },
                      ),
          ),
        ],
      ),
    );
  }
  
  List<UnifiedPageInfo> _filterHistory(List<UnifiedPageInfo> history) {
    var filtered = history;
    
    // Filter by browser
    if (_selectedBrowser != null) {
      filtered = filtered.where((h) => h.browserType == _selectedBrowser).toList();
    }
    
    // Filter by date range
    if (_dateRange != null) {
      filtered = filtered.where((h) =>
        h.lastAccessed.isAfter(_dateRange!.start) &&
        h.lastAccessed.isBefore(_dateRange!.end.add(const Duration(days: 1)))
      ).toList();
    }
    
    // Filter by search query
    if (_searchQuery.isNotEmpty) {
      final query = _searchQuery.toLowerCase();
      filtered = filtered.where((h) =>
        h.title.toLowerCase().contains(query) ||
        h.url.toLowerCase().contains(query)
      ).toList();
    }
    
    // Sort by last accessed (most recent first)
    filtered.sort((a, b) => b.lastAccessed.compareTo(a.lastAccessed));
    
    return filtered;
  }
  
  Map<String, List<UnifiedPageInfo>> _groupByDate(List<UnifiedPageInfo> history) {
    final grouped = <String, List<UnifiedPageInfo>>{};
    final now = DateTime.now();
    final today = DateTime(now.year, now.month, now.day);
    final yesterday = today.subtract(const Duration(days: 1));
    
    for (final item in history) {
      final itemDate = DateTime(
        item.lastAccessed.year,
        item.lastAccessed.month,
        item.lastAccessed.day,
      );
      
      String key;
      if (itemDate == today) {
        key = '今天';
      } else if (itemDate == yesterday) {
        key = '昨天';
      } else if (itemDate.isAfter(today.subtract(const Duration(days: 7)))) {
        key = DateFormat('EEEE', 'zh_CN').format(item.lastAccessed);
      } else {
        key = DateFormat('yyyy年M月d日').format(item.lastAccessed);
      }
      
      grouped.putIfAbsent(key, () => []).add(item);
    }
    
    return grouped;
  }
  
  Widget _buildDateGroup(String date, List<UnifiedPageInfo> items) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Padding(
          padding: const EdgeInsets.symmetric(vertical: 8),
          child: Text(
            date,
            style: Theme.of(context).textTheme.titleSmall?.copyWith(
              color: Theme.of(context).colorScheme.primary,
            ),
          ),
        ),
        ...items.map((item) => Card(
          margin: const EdgeInsets.only(bottom: 8),
          child: PageListTile(
            page: item,
            onTap: () => _restoreTab(item),
            showTime: true,
            trailing: IconButton(
              icon: const Icon(Icons.restore),
              onPressed: () => _restoreTab(item),
              tooltip: '恢复标签页',
            ),
          ),
        )),
        const SizedBox(height: 8),
      ],
    );
  }
  
  Widget _buildEmptyState() {
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(
            Icons.history,
            size: 64,
            color: Theme.of(context).colorScheme.outline,
          ),
          const SizedBox(height: 16),
          Text(
            '暂无历史记录',
            style: Theme.of(context).textTheme.bodyLarge?.copyWith(
              color: Theme.of(context).colorScheme.outline,
            ),
          ),
        ],
      ),
    );
  }
  
  Future<void> _selectDateRange(BuildContext context) async {
    final now = DateTime.now();
    final result = await showDateRangePicker(
      context: context,
      firstDate: now.subtract(const Duration(days: 365)),
      lastDate: now,
      initialDateRange: _dateRange,
    );
    
    if (result != null) {
      setState(() {
        _dateRange = result;
      });
    }
  }
  
  String _formatDateRange(DateTimeRange range) {
    final format = DateFormat('M/d');
    return '${format.format(range.start)} - ${format.format(range.end)}';
  }
  
  Future<void> _showClearHistoryDialog(BuildContext context) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('清除历史记录'),
        content: const Text('确定要清除所有历史记录吗？此操作无法撤销。'),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: const Text('取消'),
          ),
          TextButton(
            onPressed: () => Navigator.pop(context, true),
            style: TextButton.styleFrom(foregroundColor: Colors.red),
            child: const Text('清除'),
          ),
        ],
      ),
    );
    
    if (confirmed == true) {
      // TODO: Clear history via provider
    }
  }
  
  void _restoreTab(UnifiedPageInfo historyItem) {
    // TODO: Restore tab via provider
  }
}
