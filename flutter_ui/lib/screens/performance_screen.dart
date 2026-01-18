import 'dart:async';
import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../models/performance_metrics.dart';
import '../providers/settings_provider.dart';

/// Screen for displaying performance monitoring and resource management
class PerformanceScreen extends StatefulWidget {
  const PerformanceScreen({super.key});

  @override
  State<PerformanceScreen> createState() => _PerformanceScreenState();
}

class _PerformanceScreenState extends State<PerformanceScreen> {
  PerformanceSummary _summary = PerformanceSummary.empty();
  List<PerformanceMetrics> _history = [];
  Timer? _refreshTimer;
  bool _isLoading = true;

  @override
  void initState() {
    super.initState();
    _loadData();
    _startAutoRefresh();
  }

  @override
  void dispose() {
    _refreshTimer?.cancel();
    super.dispose();
  }

  void _startAutoRefresh() {
    _refreshTimer = Timer.periodic(const Duration(seconds: 5), (_) {
      _loadData();
    });
  }

  Future<void> _loadData() async {
    // TODO: Load from Rust FFI
    // For now, generate mock data
    setState(() {
      _summary = _generateMockSummary();
      _history = _generateMockHistory();
      _isLoading = false;
    });
  }

  PerformanceSummary _generateMockSummary() {
    return PerformanceSummary(
      currentMemoryMb: 256,
      currentCpuPercent: 15.5,
      avgMemoryMb: 220,
      avgCpuPercent: 12.3,
      maxMemoryMb: 380,
      maxCpuPercent: 45.2,
      avgResponseTimeMs: 85,
      totalErrors: 2,
      cacheHitRate: 0.87,
      resourceLevel: ResourceLevel.normal,
      memoryLimitMb: 512,
      cpuLimitPercent: 50.0,
      samplesCount: 120,
    );
  }

  List<PerformanceMetrics> _generateMockHistory() {
    final now = DateTime.now();
    return List.generate(10, (i) {
      return PerformanceMetrics(
        timestamp: now.subtract(Duration(minutes: i * 5)),
        memoryUsageBytes: (200 + i * 10) * 1024 * 1024,
        cpuUsagePercent: 10.0 + i * 2,
        activeConnections: 2,
        managedPages: 45 + i,
        pendingAiTasks: i % 3,
        databaseSizeBytes: 50 * 1024 * 1024,
        cacheHitRate: 0.85 + (i * 0.01),
        avgResponseTimeMs: 80 + i * 5,
        recentErrorCount: i % 2,
      );
    });
  }

  @override
  Widget build(BuildContext context) {
    final settings = context.watch<SettingsProvider>();
    final theme = Theme.of(context);

    return Scaffold(
      appBar: AppBar(
        title: const Text('性能监控'),
        actions: [
          IconButton(
            icon: const Icon(Icons.refresh),
            onPressed: _loadData,
            tooltip: '刷新',
          ),
        ],
      ),
      body: _isLoading
          ? const Center(child: CircularProgressIndicator())
          : RefreshIndicator(
              onRefresh: _loadData,
              child: ListView(
                padding: const EdgeInsets.all(16),
                children: [
                  // Resource status card
                  _buildResourceStatusCard(theme),
                  const SizedBox(height: 16),

                  // Current metrics cards
                  _buildMetricsRow(theme),
                  const SizedBox(height: 16),

                  // Performance chart placeholder
                  _buildPerformanceChart(theme),
                  const SizedBox(height: 16),

                  // Resource configuration
                  _buildResourceConfigSection(theme, settings),
                  const SizedBox(height: 16),

                  // Recent activity
                  _buildRecentActivitySection(theme),
                ],
              ),
            ),
    );
  }

  Widget _buildResourceStatusCard(ThemeData theme) {
    final statusColor = _getResourceLevelColor(_summary.resourceLevel);
    final statusIcon = _getResourceLevelIcon(_summary.resourceLevel);

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Row(
          children: [
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: statusColor.withOpacity(0.1),
                borderRadius: BorderRadius.circular(12),
              ),
              child: Icon(statusIcon, color: statusColor, size: 32),
            ),
            const SizedBox(width: 16),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    '系统资源状态',
                    style: theme.textTheme.titleMedium,
                  ),
                  const SizedBox(height: 4),
                  Text(
                    '${_summary.resourceLevel.displayName} - 资源使用正常',
                    style: theme.textTheme.bodyMedium?.copyWith(
                      color: statusColor,
                    ),
                  ),
                ],
              ),
            ),
            Column(
              crossAxisAlignment: CrossAxisAlignment.end,
              children: [
                Text(
                  '${_summary.samplesCount} 个样本',
                  style: theme.textTheme.bodySmall,
                ),
                Text(
                  '${_summary.totalErrors} 个错误',
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: _summary.totalErrors > 0 ? Colors.orange : null,
                  ),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildMetricsRow(ThemeData theme) {
    return Row(
      children: [
        Expanded(
          child: _buildMetricCard(
            theme,
            '内存使用',
            '${_summary.currentMemoryMb} MB',
            '限制: ${_summary.memoryLimitMb} MB',
            _summary.memoryUsagePercent / 100,
            Icons.memory,
          ),
        ),
        const SizedBox(width: 12),
        Expanded(
          child: _buildMetricCard(
            theme,
            'CPU 使用',
            '${_summary.currentCpuPercent.toStringAsFixed(1)}%',
            '限制: ${_summary.cpuLimitPercent.toStringAsFixed(0)}%',
            _summary.cpuUsageOfLimit / 100,
            Icons.speed,
          ),
        ),
      ],
    );
  }

  Widget _buildMetricCard(
    ThemeData theme,
    String title,
    String value,
    String subtitle,
    double progress,
    IconData icon,
  ) {
    final progressColor = progress > 0.9
        ? Colors.red
        : progress > 0.7
            ? Colors.orange
            : theme.colorScheme.primary;

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(icon, size: 20, color: theme.colorScheme.primary),
                const SizedBox(width: 8),
                Text(title, style: theme.textTheme.titleSmall),
              ],
            ),
            const SizedBox(height: 12),
            Text(
              value,
              style: theme.textTheme.headlineMedium?.copyWith(
                fontWeight: FontWeight.bold,
              ),
            ),
            const SizedBox(height: 8),
            LinearProgressIndicator(
              value: progress.clamp(0.0, 1.0),
              backgroundColor: progressColor.withOpacity(0.2),
              valueColor: AlwaysStoppedAnimation(progressColor),
            ),
            const SizedBox(height: 4),
            Text(
              subtitle,
              style: theme.textTheme.bodySmall,
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildPerformanceChart(ThemeData theme) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Text('性能趋势', style: theme.textTheme.titleMedium),
                Row(
                  children: [
                    _buildLegendItem(theme, '内存', Colors.blue),
                    const SizedBox(width: 16),
                    _buildLegendItem(theme, 'CPU', Colors.green),
                  ],
                ),
              ],
            ),
            const SizedBox(height: 16),
            SizedBox(
              height: 150,
              child: CustomPaint(
                size: const Size(double.infinity, 150),
                painter: _SimpleChartPainter(
                  memoryData:
                      _history.map((m) => m.memoryUsageMb / 512).toList(),
                  cpuData:
                      _history.map((m) => m.cpuUsagePercent / 100).toList(),
                ),
              ),
            ),
            const SizedBox(height: 8),
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Text(
                  '50分钟前',
                  style: theme.textTheme.bodySmall,
                ),
                Text(
                  '现在',
                  style: theme.textTheme.bodySmall,
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildLegendItem(ThemeData theme, String label, Color color) {
    return Row(
      children: [
        Container(
          width: 12,
          height: 12,
          decoration: BoxDecoration(
            color: color,
            borderRadius: BorderRadius.circular(2),
          ),
        ),
        const SizedBox(width: 4),
        Text(label, style: theme.textTheme.bodySmall),
      ],
    );
  }

  Widget _buildResourceConfigSection(
      ThemeData theme, SettingsProvider settings) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('资源管理配置', style: theme.textTheme.titleMedium),
            const SizedBox(height: 16),
            SwitchListTile(
              title: const Text('自适应资源管理'),
              subtitle: const Text('根据系统负载自动调整处理优先级'),
              value: settings.resourceConfig.adaptiveManagement,
              onChanged: settings.setAdaptiveManagement,
            ),
            const Divider(),
            ListTile(
              title: const Text('最大内存限制'),
              subtitle: Text('${settings.resourceConfig.maxMemoryMb} MB'),
              trailing: SizedBox(
                width: 150,
                child: Slider(
                  value: settings.resourceConfig.maxMemoryMb.toDouble(),
                  min: 256,
                  max: 2048,
                  divisions: 7,
                  label: '${settings.resourceConfig.maxMemoryMb} MB',
                  onChanged: (value) => settings.setMaxMemoryMb(value.toInt()),
                ),
              ),
            ),
            ListTile(
              title: const Text('最大 CPU 限制'),
              subtitle: Text(
                  '${settings.resourceConfig.maxCpuPercent.toStringAsFixed(0)}%'),
              trailing: SizedBox(
                width: 150,
                child: Slider(
                  value: settings.resourceConfig.maxCpuPercent,
                  min: 25,
                  max: 100,
                  divisions: 3,
                  label:
                      '${settings.resourceConfig.maxCpuPercent.toStringAsFixed(0)}%',
                  onChanged: settings.setMaxCpuPercent,
                ),
              ),
            ),
            ListTile(
              title: const Text('并发 AI 任务数'),
              subtitle:
                  Text('${settings.resourceConfig.maxConcurrentAiTasks} 个'),
              trailing: SizedBox(
                width: 150,
                child: Slider(
                  value:
                      settings.resourceConfig.maxConcurrentAiTasks.toDouble(),
                  min: 1,
                  max: 8,
                  divisions: 7,
                  label: '${settings.resourceConfig.maxConcurrentAiTasks}',
                  onChanged: (value) =>
                      settings.setMaxConcurrentAiTasks(value.toInt()),
                ),
              ),
            ),
            ListTile(
              title: const Text('缓存大小'),
              subtitle: Text('${settings.resourceConfig.cacheSizeMb} MB'),
              trailing: SizedBox(
                width: 150,
                child: Slider(
                  value: settings.resourceConfig.cacheSizeMb.toDouble(),
                  min: 50,
                  max: 500,
                  divisions: 9,
                  label: '${settings.resourceConfig.cacheSizeMb} MB',
                  onChanged: (value) => settings.setCacheSizeMb(value.toInt()),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildRecentActivitySection(ThemeData theme) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('最近活动', style: theme.textTheme.titleMedium),
            const SizedBox(height: 16),
            ..._history.take(5).map((m) => _buildActivityItem(theme, m)),
          ],
        ),
      ),
    );
  }

  Widget _buildActivityItem(ThemeData theme, PerformanceMetrics metrics) {
    final timeAgo = DateTime.now().difference(metrics.timestamp);
    final timeText = timeAgo.inMinutes < 1 ? '刚刚' : '${timeAgo.inMinutes} 分钟前';

    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: Row(
        children: [
          Container(
            width: 8,
            height: 8,
            decoration: BoxDecoration(
              color:
                  metrics.recentErrorCount > 0 ? Colors.orange : Colors.green,
              shape: BoxShape.circle,
            ),
          ),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  '内存: ${metrics.memoryUsageMb.toStringAsFixed(0)} MB, '
                  'CPU: ${metrics.cpuUsagePercent.toStringAsFixed(1)}%',
                  style: theme.textTheme.bodyMedium,
                ),
                Text(
                  '${metrics.managedPages} 页面, ${metrics.pendingAiTasks} AI任务',
                  style: theme.textTheme.bodySmall,
                ),
              ],
            ),
          ),
          Text(
            timeText,
            style: theme.textTheme.bodySmall,
          ),
        ],
      ),
    );
  }

  Color _getResourceLevelColor(ResourceLevel level) {
    switch (level) {
      case ResourceLevel.low:
        return Colors.green;
      case ResourceLevel.normal:
        return Colors.blue;
      case ResourceLevel.high:
        return Colors.orange;
      case ResourceLevel.critical:
        return Colors.red;
    }
  }

  IconData _getResourceLevelIcon(ResourceLevel level) {
    switch (level) {
      case ResourceLevel.low:
        return Icons.check_circle;
      case ResourceLevel.normal:
        return Icons.info;
      case ResourceLevel.high:
        return Icons.warning;
      case ResourceLevel.critical:
        return Icons.error;
    }
  }
}

/// Simple chart painter for performance visualization
class _SimpleChartPainter extends CustomPainter {
  final List<double> memoryData;
  final List<double> cpuData;

  _SimpleChartPainter({
    required this.memoryData,
    required this.cpuData,
  });

  @override
  void paint(Canvas canvas, Size size) {
    if (memoryData.isEmpty || cpuData.isEmpty) return;

    final memoryPaint = Paint()
      ..color = Colors.blue
      ..strokeWidth = 2
      ..style = PaintingStyle.stroke;

    final cpuPaint = Paint()
      ..color = Colors.green
      ..strokeWidth = 2
      ..style = PaintingStyle.stroke;

    final gridPaint = Paint()
      ..color = Colors.grey.withOpacity(0.2)
      ..strokeWidth = 1;

    // Draw grid lines
    for (var i = 0; i <= 4; i++) {
      final y = size.height * i / 4;
      canvas.drawLine(Offset(0, y), Offset(size.width, y), gridPaint);
    }

    // Draw memory line
    _drawLine(canvas, size, memoryData, memoryPaint);

    // Draw CPU line
    _drawLine(canvas, size, cpuData, cpuPaint);
  }

  void _drawLine(Canvas canvas, Size size, List<double> data, Paint paint) {
    if (data.length < 2) return;

    final path = Path();
    final stepX = size.width / (data.length - 1);

    for (var i = 0; i < data.length; i++) {
      final x = i * stepX;
      final y = size.height * (1 - data[i].clamp(0.0, 1.0));

      if (i == 0) {
        path.moveTo(x, y);
      } else {
        path.lineTo(x, y);
      }
    }

    canvas.drawPath(path, paint);
  }

  @override
  bool shouldRepaint(covariant _SimpleChartPainter oldDelegate) {
    return memoryData != oldDelegate.memoryData ||
        cpuData != oldDelegate.cpuData;
  }
}
