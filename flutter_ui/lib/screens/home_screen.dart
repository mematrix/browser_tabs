import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';
import 'package:provider/provider.dart';

import '../providers/page_provider.dart';
import '../widgets/stats_card.dart';
import '../widgets/page_list_tile.dart';
import '../widgets/group_card.dart';

/// Home screen with overview and quick access
class HomeScreen extends StatelessWidget {
  const HomeScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final pageProvider = context.watch<PageProvider>();
    final stats = pageProvider.stats;
    
    return Scaffold(
      appBar: AppBar(
        title: const Text('Browser Page Manager'),
        actions: [
          IconButton(
            icon: const Icon(Icons.refresh),
            onPressed: pageProvider.isLoading ? null : pageProvider.refresh,
            tooltip: '刷新',
          ),
        ],
      ),
      body: pageProvider.isLoading
          ? const Center(child: CircularProgressIndicator())
          : RefreshIndicator(
              onRefresh: pageProvider.refresh,
              child: SingleChildScrollView(
                padding: const EdgeInsets.all(16),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    // Stats overview
                    _buildStatsSection(context, stats),
                    const SizedBox(height: 24),
                    
                    // Quick actions
                    _buildQuickActions(context),
                    const SizedBox(height: 24),
                    
                    // Recent tabs
                    _buildRecentTabs(context, pageProvider),
                    const SizedBox(height: 24),
                    
                    // Smart groups
                    _buildSmartGroups(context, pageProvider),
                  ],
                ),
              ),
            ),
    );
  }
  
  Widget _buildStatsSection(BuildContext context, PageStats stats) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          '概览',
          style: Theme.of(context).textTheme.titleLarge,
        ),
        const SizedBox(height: 12),
        Wrap(
          spacing: 12,
          runSpacing: 12,
          children: [
            StatsCard(
              icon: Icons.tab,
              label: '活跃标签页',
              value: stats.totalTabs.toString(),
              color: Colors.blue,
            ),
            StatsCard(
              icon: Icons.bookmark,
              label: '书签',
              value: stats.totalBookmarks.toString(),
              color: Colors.orange,
            ),
            StatsCard(
              icon: Icons.history,
              label: '历史记录',
              value: stats.totalHistory.toString(),
              color: Colors.purple,
            ),
            StatsCard(
              icon: Icons.folder,
              label: '智能分组',
              value: stats.totalGroups.toString(),
              color: Colors.green,
            ),
            StatsCard(
              icon: Icons.computer,
              label: '已连接浏览器',
              value: stats.connectedBrowsers.toString(),
              color: Colors.teal,
            ),
          ],
        ),
      ],
    );
  }
  
  Widget _buildQuickActions(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          '快速操作',
          style: Theme.of(context).textTheme.titleLarge,
        ),
        const SizedBox(height: 12),
        Wrap(
          spacing: 8,
          runSpacing: 8,
          children: [
            ActionChip(
              avatar: const Icon(Icons.search, size: 18),
              label: const Text('搜索'),
              onPressed: () => context.go('/search'),
            ),
            ActionChip(
              avatar: const Icon(Icons.sync, size: 18),
              label: const Text('同步书签'),
              onPressed: () {
                // TODO: Trigger bookmark sync
              },
            ),
            ActionChip(
              avatar: const Icon(Icons.auto_awesome, size: 18),
              label: const Text('智能分组'),
              onPressed: () {
                // TODO: Trigger AI grouping
              },
            ),
            ActionChip(
              avatar: const Icon(Icons.cleaning_services, size: 18),
              label: const Text('清理重复'),
              onPressed: () {
                // TODO: Show duplicate cleanup
              },
            ),
          ],
        ),
      ],
    );
  }
  
  Widget _buildRecentTabs(BuildContext context, PageProvider provider) {
    final recentTabs = provider.activeTabs.take(5).toList();
    
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          mainAxisAlignment: MainAxisAlignment.spaceBetween,
          children: [
            Text(
              '最近标签页',
              style: Theme.of(context).textTheme.titleLarge,
            ),
            TextButton(
              onPressed: () => context.go('/tabs'),
              child: const Text('查看全部'),
            ),
          ],
        ),
        const SizedBox(height: 8),
        if (recentTabs.isEmpty)
          const Card(
            child: Padding(
              padding: EdgeInsets.all(16),
              child: Text('暂无活跃标签页'),
            ),
          )
        else
          Card(
            child: Column(
              children: recentTabs.map((tab) => PageListTile(
                page: tab,
                onTap: () => provider.activateTab(tab),
                onClose: () => provider.closeTab(tab),
                onBookmark: tab.hasBookmark ? null : () => provider.createBookmarkFromTab(tab),
              )).toList(),
            ),
          ),
      ],
    );
  }
  
  Widget _buildSmartGroups(BuildContext context, PageProvider provider) {
    final groups = provider.groups.take(4).toList();
    
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          mainAxisAlignment: MainAxisAlignment.spaceBetween,
          children: [
            Text(
              '智能分组',
              style: Theme.of(context).textTheme.titleLarge,
            ),
            TextButton(
              onPressed: () {
                // TODO: Navigate to groups view
              },
              child: const Text('查看全部'),
            ),
          ],
        ),
        const SizedBox(height: 8),
        if (groups.isEmpty)
          const Card(
            child: Padding(
              padding: EdgeInsets.all(16),
              child: Text('暂无智能分组'),
            ),
          )
        else
          Wrap(
            spacing: 12,
            runSpacing: 12,
            children: groups.map((group) => GroupCard(
              group: group,
              onTap: () {
                // TODO: Show group details
              },
            )).toList(),
          ),
      ],
    );
  }
}
