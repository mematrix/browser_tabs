# Web Page Manager - Flutter UI

Cross-platform Flutter UI for the Web Page Manager application.

## Features

- **Unified Tab/Bookmark View**: View and manage tabs and bookmarks from all connected browsers in one place
- **Smart Search**: Full-text search across tabs, bookmarks, history, and archived content
- **Smart Groups**: AI-powered content grouping and organization
- **System Tray**: Minimize to system tray with quick access menu
- **Global Hotkeys**: Keyboard shortcuts for quick access
- **Native Notifications**: System notifications for tab activity and sync events
- **Cross-Platform**: Works on Windows, Linux, and macOS

## Project Structure

```
flutter_ui/
├── lib/
│   ├── main.dart              # Application entry point
│   ├── app.dart               # Main app widget and routing
│   ├── models/                # Data models
│   │   ├── page_info.dart     # Unified page information
│   │   ├── smart_group.dart   # Smart group model
│   │   └── search_result.dart # Search result models
│   ├── providers/             # State management
│   │   ├── page_provider.dart     # Page data provider
│   │   ├── search_provider.dart   # Search state provider
│   │   └── settings_provider.dart # Settings provider
│   ├── screens/               # UI screens
│   │   ├── home_screen.dart       # Home/overview screen
│   │   ├── tabs_screen.dart       # Tabs management screen
│   │   ├── bookmarks_screen.dart  # Bookmarks screen
│   │   ├── search_screen.dart     # Search screen
│   │   ├── history_screen.dart    # History screen
│   │   └── settings_screen.dart   # Settings screen
│   ├── services/              # Platform services
│   │   ├── rust_bridge.dart       # Rust FFI bridge
│   │   ├── system_tray_service.dart
│   │   ├── notification_service.dart
│   │   └── hotkey_service.dart
│   ├── widgets/               # Reusable widgets
│   │   ├── stats_card.dart
│   │   ├── page_list_tile.dart
│   │   ├── group_card.dart
│   │   ├── browser_filter_chips.dart
│   │   └── search_result_tile.dart
│   └── theme/                 # Theme configuration
│       └── app_theme.dart
├── assets/                    # Static assets
│   ├── icons/
│   ├── images/
│   └── fonts/
└── pubspec.yaml              # Flutter dependencies
```

## Dependencies

### Core Dependencies
- `provider` - State management
- `go_router` - Navigation and routing
- `window_manager` - Window management for desktop

### System Integration
- `system_tray` - System tray support
- `local_notifier` - Native notifications
- `hotkey_manager` - Global hotkey registration

### UI Components
- `cached_network_image` - Image caching
- `flutter_svg` - SVG support

### Data & Utilities
- `ffi` - Rust FFI integration
- `shared_preferences` - Local storage
- `url_launcher` - URL handling
- `intl` - Internationalization

## Getting Started

### Prerequisites

1. Flutter SDK (3.0.0 or later)
2. Rust toolchain (for building the core library)
3. Platform-specific requirements:
   - **Windows**: Visual Studio with C++ workload
   - **Linux**: GTK development libraries
   - **macOS**: Xcode command line tools

### Building

```bash
# Get dependencies
flutter pub get

# Run in debug mode
flutter run -d windows  # or linux, macos

# Build release
flutter build windows  # or linux, macos
```

### Development

The Flutter UI communicates with the Rust core via FFI. During development, mock data is provided by `RustBridge` for UI testing without the full Rust backend.

To enable full Rust integration:
1. Build the Rust core library
2. Place the dynamic library in the appropriate location
3. Update `RustBridge` to load and call the actual FFI functions

## Architecture

### State Management

The app uses Provider for state management:
- `PageProvider` - Manages page data (tabs, bookmarks, groups)
- `SearchProvider` - Manages search state and results
- `SettingsProvider` - Manages application settings

### Navigation

Navigation is handled by `go_router` with a shell route pattern:
- Main shell provides the navigation rail
- Child routes render in the content area

### Rust Integration

The `RustBridge` service handles communication with the Rust core:
- FFI calls for data retrieval
- Method channels for real-time updates
- Async operations with proper error handling

## Requirements Implemented

- **Requirement 4.1**: Flutter cross-platform UI with consistent experience
- **Requirement 4.2**: System tray with quick access functionality
- **Requirement 4.3**: Native notifications for tab activity
- **Requirement 6.5**: Unified search across tabs and bookmarks
