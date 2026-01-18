import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';
import 'package:provider/provider.dart';

import 'providers/settings_provider.dart';
import 'screens/home_screen.dart';
import 'screens/tabs_screen.dart';
import 'screens/bookmarks_screen.dart';
import 'screens/search_screen.dart';
import 'screens/history_screen.dart';
import 'screens/settings_screen.dart';
import 'theme/app_theme.dart';

class WebPageManagerApp extends StatelessWidget {
  const WebPageManagerApp({super.key});

  @override
  Widget build(BuildContext context) {
    final settings = context.watch<SettingsProvider>();

    return MaterialApp.router(
      title: 'Web Page Manager',
      debugShowCheckedModeBanner: false,
      theme: AppTheme.lightTheme,
      darkTheme: AppTheme.darkTheme,
      themeMode: settings.themeMode,
      routerConfig: _router,
    );
  }

  static final _router = GoRouter(
    initialLocation: '/',
    routes: [
      ShellRoute(
        builder: (context, state, child) => MainShell(child: child),
        routes: [
          GoRoute(
            path: '/',
            builder: (context, state) => const HomeScreen(),
          ),
          GoRoute(
            path: '/tabs',
            builder: (context, state) => const TabsScreen(),
          ),
          GoRoute(
            path: '/bookmarks',
            builder: (context, state) => const BookmarksScreen(),
          ),
          GoRoute(
            path: '/search',
            builder: (context, state) => const SearchScreen(),
          ),
          GoRoute(
            path: '/history',
            builder: (context, state) => const HistoryScreen(),
          ),
          GoRoute(
            path: '/settings',
            builder: (context, state) => const SettingsScreen(),
          ),
        ],
      ),
    ],
  );
}

/// Main shell with navigation rail
class MainShell extends StatelessWidget {
  final Widget child;

  const MainShell({super.key, required this.child});

  @override
  Widget build(BuildContext context) {
    final currentPath = GoRouterState.of(context).uri.path;

    return Scaffold(
      body: Row(
        children: [
          NavigationRail(
            selectedIndex: _getSelectedIndex(currentPath),
            onDestinationSelected: (index) =>
                _onDestinationSelected(context, index),
            labelType: NavigationRailLabelType.all,
            leading: Padding(
              padding: const EdgeInsets.symmetric(vertical: 16),
              child: Icon(
                Icons.web,
                size: 32,
                color: Theme.of(context).colorScheme.primary,
              ),
            ),
            destinations: const [
              NavigationRailDestination(
                icon: Icon(Icons.home_outlined),
                selectedIcon: Icon(Icons.home),
                label: Text('首页'),
              ),
              NavigationRailDestination(
                icon: Icon(Icons.tab_outlined),
                selectedIcon: Icon(Icons.tab),
                label: Text('标签页'),
              ),
              NavigationRailDestination(
                icon: Icon(Icons.bookmark_outline),
                selectedIcon: Icon(Icons.bookmark),
                label: Text('书签'),
              ),
              NavigationRailDestination(
                icon: Icon(Icons.search_outlined),
                selectedIcon: Icon(Icons.search),
                label: Text('搜索'),
              ),
              NavigationRailDestination(
                icon: Icon(Icons.history_outlined),
                selectedIcon: Icon(Icons.history),
                label: Text('历史'),
              ),
              NavigationRailDestination(
                icon: Icon(Icons.settings_outlined),
                selectedIcon: Icon(Icons.settings),
                label: Text('设置'),
              ),
            ],
          ),
          const VerticalDivider(thickness: 1, width: 1),
          Expanded(child: child),
        ],
      ),
    );
  }

  int _getSelectedIndex(String path) {
    switch (path) {
      case '/':
        return 0;
      case '/tabs':
        return 1;
      case '/bookmarks':
        return 2;
      case '/search':
        return 3;
      case '/history':
        return 4;
      case '/settings':
        return 5;
      default:
        return 0;
    }
  }

  void _onDestinationSelected(BuildContext context, int index) {
    final paths = [
      '/',
      '/tabs',
      '/bookmarks',
      '/search',
      '/history',
      '/settings'
    ];
    context.go(paths[index]);
  }
}
