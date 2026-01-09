import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../providers/settings_provider.dart';
import 'performance_screen.dart';

/// Screen for application settings
class SettingsScreen extends StatelessWidget {
  const SettingsScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final settings = context.watch<SettingsProvider>();
    
    return Scaffold(
      appBar: AppBar(
        title: const Text('设置'),
      ),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          // Appearance section
          _buildSectionHeader(context, '外观'),
          Card(
            child: Column(
              children: [
                ListTile(
                  leading: const Icon(Icons.brightness_6),
                  title: const Text('主题'),
                  trailing: DropdownButton<ThemeMode>(
                    value: settings.themeMode,
                    underline: const SizedBox(),
                    items: const [
                      DropdownMenuItem(
                        value: ThemeMode.system,
                        child: Text('跟随系统'),
                      ),
                      DropdownMenuItem(
                        value: ThemeMode.light,
                        child: Text('浅色'),
                      ),
                      DropdownMenuItem(
                        value: ThemeMode.dark,
                        child: Text('深色'),
                      ),
                    ],
                    onChanged: (mode) {
                      if (mode != null) {
                        settings.setThemeMode(mode);
                      }
                    },
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: 16),
          
          // Behavior section
          _buildSectionHeader(context, '行为'),
          Card(
            child: Column(
              children: [
                SwitchListTile(
                  secondary: const Icon(Icons.minimize),
                  title: const Text('最小化到托盘'),
                  subtitle: const Text('关闭窗口时最小化到系统托盘'),
                  value: settings.minimizeToTray,
                  onChanged: settings.setMinimizeToTray,
                ),
                const Divider(height: 1),
                SwitchListTile(
                  secondary: const Icon(Icons.notifications),
                  title: const Text('显示通知'),
                  subtitle: const Text('显示标签页活动和同步通知'),
                  value: settings.showNotifications,
                  onChanged: settings.setShowNotifications,
                ),
                const Divider(height: 1),
                SwitchListTile(
                  secondary: const Icon(Icons.keyboard),
                  title: const Text('全局热键'),
                  subtitle: const Text('启用全局键盘快捷键'),
                  value: settings.enableHotkeys,
                  onChanged: settings.setEnableHotkeys,
                ),
              ],
            ),
          ),
          const SizedBox(height: 16),
          
          // Data section
          _buildSectionHeader(context, '数据'),
          Card(
            child: Column(
              children: [
                SwitchListTile(
                  secondary: const Icon(Icons.sync),
                  title: const Text('自动刷新'),
                  subtitle: const Text('自动刷新标签页和书签数据'),
                  value: settings.autoRefresh,
                  onChanged: settings.setAutoRefresh,
                ),
                if (settings.autoRefresh) ...[
                  const Divider(height: 1),
                  ListTile(
                    leading: const Icon(Icons.timer),
                    title: const Text('刷新间隔'),
                    trailing: DropdownButton<int>(
                      value: settings.autoRefreshInterval,
                      underline: const SizedBox(),
                      items: const [
                        DropdownMenuItem(value: 15, child: Text('15 秒')),
                        DropdownMenuItem(value: 30, child: Text('30 秒')),
                        DropdownMenuItem(value: 60, child: Text('1 分钟')),
                        DropdownMenuItem(value: 300, child: Text('5 分钟')),
                      ],
                      onChanged: (value) {
                        if (value != null) {
                          settings.setAutoRefreshInterval(value);
                        }
                      },
                    ),
                  ),
                ],
              ],
            ),
          ),
          const SizedBox(height: 16),
          
          // Browser section
          _buildSectionHeader(context, '浏览器'),
          Card(
            child: Column(
              children: [
                ListTile(
                  leading: const Icon(Icons.web),
                  title: const Text('默认浏览器'),
                  subtitle: const Text('用于打开链接和恢复标签页'),
                  trailing: DropdownButton<String>(
                    value: settings.defaultBrowser,
                    underline: const SizedBox(),
                    items: const [
                      DropdownMenuItem(value: 'chrome', child: Text('Chrome')),
                      DropdownMenuItem(value: 'firefox', child: Text('Firefox')),
                      DropdownMenuItem(value: 'edge', child: Text('Edge')),
                    ],
                    onChanged: (value) {
                      if (value != null) {
                        settings.setDefaultBrowser(value);
                      }
                    },
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: 16),
          
          // Performance section
          _buildSectionHeader(context, '性能'),
          Card(
            child: Column(
              children: [
                SwitchListTile(
                  secondary: const Icon(Icons.speed),
                  title: const Text('性能监控'),
                  subtitle: const Text('启用应用性能监控和资源管理'),
                  value: settings.enablePerformanceMonitoring,
                  onChanged: settings.setEnablePerformanceMonitoring,
                ),
                if (settings.enablePerformanceMonitoring) ...[
                  const Divider(height: 1),
                  ListTile(
                    leading: const Icon(Icons.analytics),
                    title: const Text('性能监控面板'),
                    subtitle: const Text('查看详细性能数据和资源配置'),
                    trailing: const Icon(Icons.chevron_right),
                    onTap: () {
                      Navigator.push(
                        context,
                        MaterialPageRoute(
                          builder: (context) => const PerformanceScreen(),
                        ),
                      );
                    },
                  ),
                  const Divider(height: 1),
                  SwitchListTile(
                    secondary: const Icon(Icons.auto_fix_high),
                    title: const Text('自适应资源管理'),
                    subtitle: const Text('根据系统负载自动调整处理优先级'),
                    value: settings.resourceConfig.adaptiveManagement,
                    onChanged: settings.setAdaptiveManagement,
                  ),
                  const Divider(height: 1),
                  ListTile(
                    leading: const Icon(Icons.history),
                    title: const Text('历史记录保留'),
                    trailing: DropdownButton<int>(
                      value: settings.performanceHistoryHours,
                      underline: const SizedBox(),
                      items: const [
                        DropdownMenuItem(value: 6, child: Text('6 小时')),
                        DropdownMenuItem(value: 12, child: Text('12 小时')),
                        DropdownMenuItem(value: 24, child: Text('24 小时')),
                        DropdownMenuItem(value: 48, child: Text('48 小时')),
                      ],
                      onChanged: (value) {
                        if (value != null) {
                          settings.setPerformanceHistoryHours(value);
                        }
                      },
                    ),
                  ),
                ],
              ],
            ),
          ),
          const SizedBox(height: 16),
          
          // Hotkeys section
          if (settings.enableHotkeys) ...[
            _buildSectionHeader(context, '快捷键'),
            Card(
              child: Column(
                children: [
                  ListTile(
                    leading: const Icon(Icons.search),
                    title: const Text('快速搜索'),
                    trailing: Text(
                      'Ctrl+Shift+F',
                      style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                        fontFamily: 'monospace',
                      ),
                    ),
                  ),
                  const Divider(height: 1),
                  ListTile(
                    leading: const Icon(Icons.window),
                    title: const Text('显示窗口'),
                    trailing: Text(
                      'Ctrl+Shift+W',
                      style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                        fontFamily: 'monospace',
                      ),
                    ),
                  ),
                  const Divider(height: 1),
                  ListTile(
                    leading: const Icon(Icons.add),
                    title: const Text('新建标签页'),
                    trailing: Text(
                      'Ctrl+Shift+T',
                      style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                        fontFamily: 'monospace',
                      ),
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(height: 16),
          ],
          
          // About section
          _buildSectionHeader(context, '关于'),
          Card(
            child: Column(
              children: [
                ListTile(
                  leading: const Icon(Icons.info),
                  title: const Text('版本'),
                  trailing: const Text('1.0.0'),
                ),
                const Divider(height: 1),
                ListTile(
                  leading: const Icon(Icons.restore),
                  title: const Text('重置设置'),
                  onTap: () => _showResetDialog(context, settings),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
  
  Widget _buildSectionHeader(BuildContext context, String title) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Text(
        title,
        style: Theme.of(context).textTheme.titleMedium?.copyWith(
          color: Theme.of(context).colorScheme.primary,
        ),
      ),
    );
  }
  
  Future<void> _showResetDialog(BuildContext context, SettingsProvider settings) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('重置设置'),
        content: const Text('确定要将所有设置重置为默认值吗？'),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: const Text('取消'),
          ),
          TextButton(
            onPressed: () => Navigator.pop(context, true),
            child: const Text('重置'),
          ),
        ],
      ),
    );
    
    if (confirmed == true) {
      await settings.resetToDefaults();
    }
  }
}
