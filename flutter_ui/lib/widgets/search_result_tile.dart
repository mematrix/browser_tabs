import 'package:flutter/material.dart';

import '../models/search_result.dart';

/// List tile widget for displaying search results
class SearchResultTile extends StatelessWidget {
  final SearchResultItem result;
  final VoidCallback? onTap;

  const SearchResultTile({
    super.key,
    required this.result,
    this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    return ListTile(
      leading: _buildLeading(context),
      title: Text(
        result.title,
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
      ),
      subtitle: _buildSubtitle(context),
      trailing: _buildTrailing(context),
      onTap: onTap,
    );
  }

  Widget _buildLeading(BuildContext context) {
    return Container(
      width: 40,
      height: 40,
      decoration: BoxDecoration(
        color: _getSourceColor().withOpacity(0.1),
        borderRadius: BorderRadius.circular(8),
      ),
      child: Icon(
        _getSourceIcon(),
        color: _getSourceColor(),
        size: 20,
      ),
    );
  }

  Widget _buildSubtitle(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        // URL
        Text(
          result.domain,
          style: Theme.of(context).textTheme.bodySmall,
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
        ),
        // Snippet
        if (result.snippet != null) ...[
          const SizedBox(height: 4),
          Text(
            result.snippet!,
            style: Theme.of(context).textTheme.bodySmall?.copyWith(
                  color: Theme.of(context).colorScheme.outline,
                ),
            maxLines: 2,
            overflow: TextOverflow.ellipsis,
          ),
        ],
        // Keywords
        if (result.keywords.isNotEmpty) ...[
          const SizedBox(height: 4),
          Wrap(
            spacing: 4,
            runSpacing: 4,
            children: result.keywords
                .take(3)
                .map((keyword) => Container(
                      padding: const EdgeInsets.symmetric(
                          horizontal: 6, vertical: 2),
                      decoration: BoxDecoration(
                        color: Theme.of(context)
                            .colorScheme
                            .surfaceContainerHighest,
                        borderRadius: BorderRadius.circular(4),
                      ),
                      child: Text(
                        keyword,
                        style: Theme.of(context).textTheme.labelSmall,
                      ),
                    ))
                .toList(),
          ),
        ],
      ],
    );
  }

  Widget _buildTrailing(BuildContext context) {
    return Column(
      mainAxisAlignment: MainAxisAlignment.center,
      crossAxisAlignment: CrossAxisAlignment.end,
      children: [
        // Source badge
        Container(
          padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
          decoration: BoxDecoration(
            color: _getSourceColor().withOpacity(0.1),
            borderRadius: BorderRadius.circular(4),
          ),
          child: Text(
            _getSourceLabel(),
            style: Theme.of(context).textTheme.labelSmall?.copyWith(
                  color: _getSourceColor(),
                ),
          ),
        ),
        const SizedBox(height: 4),
        // Relevance score
        Text(
          '${(result.relevanceScore * 100).toInt()}%',
          style: Theme.of(context).textTheme.labelSmall?.copyWith(
                color: Theme.of(context).colorScheme.outline,
              ),
        ),
      ],
    );
  }

  IconData _getSourceIcon() {
    switch (result.source) {
      case SearchResultSource.activeTab:
        return Icons.tab;
      case SearchResultSource.bookmark:
        return Icons.bookmark;
      case SearchResultSource.history:
        return Icons.history;
      case SearchResultSource.archive:
        return Icons.archive;
      case SearchResultSource.unifiedPage:
        return Icons.web;
    }
  }

  Color _getSourceColor() {
    switch (result.source) {
      case SearchResultSource.activeTab:
        return Colors.blue;
      case SearchResultSource.bookmark:
        return Colors.orange;
      case SearchResultSource.history:
        return Colors.purple;
      case SearchResultSource.archive:
        return Colors.green;
      case SearchResultSource.unifiedPage:
        return Colors.grey;
    }
  }

  String _getSourceLabel() {
    switch (result.source) {
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
}
