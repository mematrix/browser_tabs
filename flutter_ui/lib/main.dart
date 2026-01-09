import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:window_manager/window_manager.dart';

import 'app.dart';
import 'providers/page_provider.dart';
import 'providers/search_provider.dart';
import 'providers/settings_provider.dart';
import 'services/rust_bridge.dart';
import 'services/system_tray_service.dart';
import 'services/notification_service.dart';
import 'services/hotkey_service.dart';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();
  
  // Initialize window manager for desktop platforms
  await windowManager.ensureInitialized();
  
  const windowOptions = WindowOptions(
    size: Size(1200, 800),
    minimumSize: Size(800, 600),
    center: true,
    backgroundColor: Colors.transparent,
    skipTaskbar: false,
    titleBarStyle: TitleBarStyle.normal,
    title: 'Web Page Manager',
  );
  
  await windowManager.waitUntilReadyToShow(windowOptions, () async {
    await windowManager.show();
    await windowManager.focus();
  });
  
  // Initialize services
  final rustBridge = RustBridge();
  await rustBridge.initialize();
  
  final systemTrayService = SystemTrayService();
  await systemTrayService.initialize();
  
  final notificationService = NotificationService();
  await notificationService.initialize();
  
  final hotkeyService = HotkeyService();
  await hotkeyService.initialize();
  
  runApp(
    MultiProvider(
      providers: [
        ChangeNotifierProvider(create: (_) => PageProvider(rustBridge)),
        ChangeNotifierProvider(create: (_) => SearchProvider(rustBridge)),
        ChangeNotifierProvider(create: (_) => SettingsProvider()),
        Provider.value(value: rustBridge),
        Provider.value(value: systemTrayService),
        Provider.value(value: notificationService),
        Provider.value(value: hotkeyService),
      ],
      child: const WebPageManagerApp(),
    ),
  );
}
