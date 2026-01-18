import 'package:flutter/material.dart';

import '../models/page_info.dart';

/// Widget for filtering by browser type
class BrowserFilterChips extends StatelessWidget {
  final BrowserType? selectedBrowser;
  final ValueChanged<BrowserType?> onBrowserSelected;

  const BrowserFilterChips({
    super.key,
    required this.selectedBrowser,
    required this.onBrowserSelected,
  });

  @override
  Widget build(BuildContext context) {
    return SingleChildScrollView(
      scrollDirection: Axis.horizontal,
      child: Row(
        children: [
          FilterChip(
            label: const Text('全部'),
            selected: selectedBrowser == null,
            onSelected: (_) => onBrowserSelected(null),
          ),
          const SizedBox(width: 8),
          _buildBrowserChip(
            context,
            BrowserType.chrome,
            'Chrome',
            Colors.blue,
          ),
          const SizedBox(width: 8),
          _buildBrowserChip(
            context,
            BrowserType.firefox,
            'Firefox',
            Colors.orange,
          ),
          const SizedBox(width: 8),
          _buildBrowserChip(
            context,
            BrowserType.edge,
            'Edge',
            Colors.teal,
          ),
          const SizedBox(width: 8),
          _buildBrowserChip(
            context,
            BrowserType.safari,
            'Safari',
            Colors.blue.shade300,
          ),
        ],
      ),
    );
  }

  Widget _buildBrowserChip(
    BuildContext context,
    BrowserType browser,
    String label,
    Color color,
  ) {
    return FilterChip(
      avatar: Container(
        width: 12,
        height: 12,
        decoration: BoxDecoration(
          color: color,
          shape: BoxShape.circle,
        ),
      ),
      label: Text(label),
      selected: selectedBrowser == browser,
      onSelected: (_) => onBrowserSelected(
        selectedBrowser == browser ? null : browser,
      ),
    );
  }
}
