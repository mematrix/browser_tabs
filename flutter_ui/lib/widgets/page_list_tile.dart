import 'package:flutter/material.dart';
import 'package:intl/intl.dart';

import '../models/page_info.dart';

/// List tile widget for displaying page information
class PageListTile extends StatelessWidget {
  final UnifiedPageInfo page;
  final VoidCallback? onTap;
  final VoidCallback? onClose;
  final VoidCallback? onBookmark;
  final bool showCategory;
  final bool showTime;
  final Widget? trailing;
  
  const PageListTile({
    super.key,
    required this.page,
    this.onTap,
    this.onClose,
    this.onBookmark,
    this.showCategory = false,
    this.showTime = false,
    this.trailing,
  });

  @override
  Widget build(BuildContext context) {
    return ListTile(
      leading: _buildLeading(context),
      title: Text(
        page.title,
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
      ),
      subtitle: _buildSubtitle(context),
      trailing: trailing ?? _buildTrailing(context),
      onTap: onTap,
    );
  }
  
  Widget _buildLeading(BuildContext context) {
    return Stack(
      children: [
        Container(
          width: 40,
          height: 40,
          decoration: BoxDecoration(
            color: Theme.of(context).colorScheme.surfaceContainerHighest,
            borderRadius: BorderRadius.circular(8),
          ),
          child: page.faviconUrl != null
              ? ClipRRect(
                  borderRadius: BorderRadius.circular(8),
                  child: Image.network(
                    page.faviconUrl!,
                    width: 40,
                    height: 40,
                    fit: BoxFit.cover,
                    errorBuilder: (_, __, ___) => const Icon(Icons.web, size: 24),
                  ),
                )
              : const Icon(Icons.web, size: 24),
        ),
        if (page.browserType != null)
          Positioned(
            right: -2,
            bottom: -2,
            child: Container(
              width: 16,
              height: 16,
              decoration: BoxDecoration(
                color: _getBrowserColor(page.browserType!),
                shape: BoxShape.circle,
                border: Border.all(
                  color: Theme.of(context).colorScheme.surface,
                  width: 2,
                ),
              ),
            ),
          ),
      ],
    );
  }
  
  Widget _buildSubtitle(BuildContext context) {
    final parts = <Widget>[];
    
    // Domain
    parts.add(Text(
      page.domain,
      style: Theme.of(context).textTheme.bodySmall,
    ));
    
    // Category
    if (showCategory && page.category != null) {
      parts.add(const SizedBox(width: 8));
      parts.add(Container(
        padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
        decoration: BoxDecoration(
          color: Theme.of(context).colorScheme.primaryContainer,
          borderRadius: BorderRadius.circular(4),
        ),
        child: Text(
          page.category!,
          style: Theme.of(context).textTheme.labelSmall?.copyWith(
            color: Theme.of(context).colorScheme.onPrimaryContainer,
          ),
        ),
      ));
    }
    
    // Time
    if (showTime) {
      parts.add(const SizedBox(width: 8));
      parts.add(Text(
        _formatTime(page.lastAccessed),
        style: Theme.of(context).textTheme.bodySmall?.copyWith(
          color: Theme.of(context).colorScheme.outline,
        ),
      ));
    }
    
    // Bookmark indicator
    if (page.hasBookmark) {
      parts.add(const SizedBox(width: 8));
      parts.add(Icon(
        Icons.bookmark,
        size: 14,
        color: Theme.of(context).colorScheme.primary,
      ));
    }
    
    // Pending changes indicator
    if (page.hasPendingChanges) {
      parts.add(const SizedBox(width: 4));
      parts.add(Icon(
        Icons.sync,
        size: 14,
        color: Theme.of(context).colorScheme.tertiary,
      ));
    }
    
    return Row(children: parts);
  }
  
  Widget? _buildTrailing(BuildContext context) {
    if (onClose == null && onBookmark == null) return null;
    
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        if (onBookmark != null)
          IconButton(
            icon: const Icon(Icons.bookmark_add_outlined),
            onPressed: onBookmark,
            tooltip: '添加书签',
            visualDensity: VisualDensity.compact,
          ),
        if (onClose != null)
          IconButton(
            icon: const Icon(Icons.close),
            onPressed: onClose,
            tooltip: '关闭',
            visualDensity: VisualDensity.compact,
          ),
      ],
    );
  }
  
  Color _getBrowserColor(BrowserType browser) {
    switch (browser) {
      case BrowserType.chrome:
        return Colors.blue;
      case BrowserType.firefox:
        return Colors.orange;
      case BrowserType.edge:
        return Colors.teal;
      case BrowserType.safari:
        return Colors.blue.shade300;
    }
  }
  
  String _formatTime(DateTime time) {
    final now = DateTime.now();
    final diff = now.difference(time);
    
    if (diff.inMinutes < 1) {
      return '刚刚';
    } else if (diff.inMinutes < 60) {
      return '${diff.inMinutes} 分钟前';
    } else if (diff.inHours < 24) {
      return '${diff.inHours} 小时前';
    } else {
      return DateFormat('M/d HH:mm').format(time);
    }
  }
}
