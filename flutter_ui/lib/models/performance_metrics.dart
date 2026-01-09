/// Performance metrics model for Flutter UI
class PerformanceMetrics {
  final DateTime timestamp;
  final int memoryUsageBytes;
  final double cpuUsagePercent;
  final int activeConnections;
  final int managedPages;
  final int pendingAiTasks;
  final int databaseSizeBytes;
  final double cacheHitRate;
  final int avgResponseTimeMs;
  final int recentErrorCount;

  PerformanceMetrics({
    required this.timestamp,
    required this.memoryUsageBytes,
    required this.cpuUsagePercent,
    required this.activeConnections,
    required this.managedPages,
    required this.pendingAiTasks,
    required this.databaseSizeBytes,
    required this.cacheHitRate,
    required this.avgResponseTimeMs,
    required this.recentErrorCount,
  });

  factory PerformanceMetrics.empty() {
    return PerformanceMetrics(
      timestamp: DateTime.now(),
      memoryUsageBytes: 0,
      cpuUsagePercent: 0.0,
      activeConnections: 0,
      managedPages: 0,
      pendingAiTasks: 0,
      databaseSizeBytes: 0,
      cacheHitRate: 0.0,
      avgResponseTimeMs: 0,
      recentErrorCount: 0,
    );
  }

  factory PerformanceMetrics.fromJson(Map<String, dynamic> json) {
    return PerformanceMetrics(
      timestamp: DateTime.parse(json['timestamp'] as String),
      memoryUsageBytes: json['memory_usage_bytes'] as int,
      cpuUsagePercent: (json['cpu_usage_percent'] as num).toDouble(),
      activeConnections: json['active_connections'] as int,
      managedPages: json['managed_pages'] as int,
      pendingAiTasks: json['pending_ai_tasks'] as int,
      databaseSizeBytes: json['database_size_bytes'] as int,
      cacheHitRate: (json['cache_hit_rate'] as num).toDouble(),
      avgResponseTimeMs: json['avg_response_time_ms'] as int,
      recentErrorCount: json['recent_error_count'] as int,
    );
  }

  Map<String, dynamic> toJson() {
    return {
      'timestamp': timestamp.toIso8601String(),
      'memory_usage_bytes': memoryUsageBytes,
      'cpu_usage_percent': cpuUsagePercent,
      'active_connections': activeConnections,
      'managed_pages': managedPages,
      'pending_ai_tasks': pendingAiTasks,
      'database_size_bytes': databaseSizeBytes,
      'cache_hit_rate': cacheHitRate,
      'avg_response_time_ms': avgResponseTimeMs,
      'recent_error_count': recentErrorCount,
    };
  }

  /// Get memory usage in MB
  double get memoryUsageMb => memoryUsageBytes / (1024 * 1024);

  /// Get database size in MB
  double get databaseSizeMb => databaseSizeBytes / (1024 * 1024);

  /// Get cache hit rate as percentage
  double get cacheHitRatePercent => cacheHitRate * 100;
}

/// Performance summary for display
class PerformanceSummary {
  final int currentMemoryMb;
  final double currentCpuPercent;
  final int avgMemoryMb;
  final double avgCpuPercent;
  final int maxMemoryMb;
  final double maxCpuPercent;
  final int avgResponseTimeMs;
  final int totalErrors;
  final double cacheHitRate;
  final ResourceLevel resourceLevel;
  final int memoryLimitMb;
  final double cpuLimitPercent;
  final int samplesCount;

  PerformanceSummary({
    required this.currentMemoryMb,
    required this.currentCpuPercent,
    required this.avgMemoryMb,
    required this.avgCpuPercent,
    required this.maxMemoryMb,
    required this.maxCpuPercent,
    required this.avgResponseTimeMs,
    required this.totalErrors,
    required this.cacheHitRate,
    required this.resourceLevel,
    required this.memoryLimitMb,
    required this.cpuLimitPercent,
    required this.samplesCount,
  });

  factory PerformanceSummary.empty() {
    return PerformanceSummary(
      currentMemoryMb: 0,
      currentCpuPercent: 0.0,
      avgMemoryMb: 0,
      avgCpuPercent: 0.0,
      maxMemoryMb: 0,
      maxCpuPercent: 0.0,
      avgResponseTimeMs: 0,
      totalErrors: 0,
      cacheHitRate: 0.0,
      resourceLevel: ResourceLevel.normal,
      memoryLimitMb: 512,
      cpuLimitPercent: 50.0,
      samplesCount: 0,
    );
  }

  factory PerformanceSummary.fromJson(Map<String, dynamic> json) {
    return PerformanceSummary(
      currentMemoryMb: json['current_memory_mb'] as int,
      currentCpuPercent: (json['current_cpu_percent'] as num).toDouble(),
      avgMemoryMb: json['avg_memory_mb'] as int,
      avgCpuPercent: (json['avg_cpu_percent'] as num).toDouble(),
      maxMemoryMb: json['max_memory_mb'] as int,
      maxCpuPercent: (json['max_cpu_percent'] as num).toDouble(),
      avgResponseTimeMs: json['avg_response_time_ms'] as int,
      totalErrors: json['total_errors'] as int,
      cacheHitRate: (json['cache_hit_rate'] as num).toDouble(),
      resourceLevel: ResourceLevel.fromString(json['resource_level'] as String),
      memoryLimitMb: json['memory_limit_mb'] as int,
      cpuLimitPercent: (json['cpu_limit_percent'] as num).toDouble(),
      samplesCount: json['samples_count'] as int,
    );
  }

  /// Get memory usage percentage of limit
  double get memoryUsagePercent => 
      memoryLimitMb > 0 ? (currentMemoryMb / memoryLimitMb) * 100 : 0;

  /// Get CPU usage percentage of limit
  double get cpuUsageOfLimit => 
      cpuLimitPercent > 0 ? (currentCpuPercent / cpuLimitPercent) * 100 : 0;
}

/// Resource usage level
enum ResourceLevel {
  low,
  normal,
  high,
  critical;

  static ResourceLevel fromString(String value) {
    switch (value.toLowerCase()) {
      case 'low':
        return ResourceLevel.low;
      case 'normal':
        return ResourceLevel.normal;
      case 'high':
        return ResourceLevel.high;
      case 'critical':
        return ResourceLevel.critical;
      default:
        return ResourceLevel.normal;
    }
  }

  String get displayName {
    switch (this) {
      case ResourceLevel.low:
        return '低';
      case ResourceLevel.normal:
        return '正常';
      case ResourceLevel.high:
        return '高';
      case ResourceLevel.critical:
        return '严重';
    }
  }
}

/// Resource configuration
class ResourceConfig {
  final int maxMemoryMb;
  final double maxCpuPercent;
  final int maxConcurrentAiTasks;
  final int maxDatabaseSizeMb;
  final bool adaptiveManagement;
  final int backgroundIntervalSecs;
  final int cacheSizeMb;

  ResourceConfig({
    required this.maxMemoryMb,
    required this.maxCpuPercent,
    required this.maxConcurrentAiTasks,
    required this.maxDatabaseSizeMb,
    required this.adaptiveManagement,
    required this.backgroundIntervalSecs,
    required this.cacheSizeMb,
  });

  factory ResourceConfig.defaults() {
    return ResourceConfig(
      maxMemoryMb: 512,
      maxCpuPercent: 50.0,
      maxConcurrentAiTasks: 4,
      maxDatabaseSizeMb: 1024,
      adaptiveManagement: true,
      backgroundIntervalSecs: 30,
      cacheSizeMb: 100,
    );
  }

  factory ResourceConfig.fromJson(Map<String, dynamic> json) {
    return ResourceConfig(
      maxMemoryMb: json['max_memory_mb'] as int,
      maxCpuPercent: (json['max_cpu_percent'] as num).toDouble(),
      maxConcurrentAiTasks: json['max_concurrent_ai_tasks'] as int,
      maxDatabaseSizeMb: json['max_database_size_mb'] as int,
      adaptiveManagement: json['adaptive_management'] as bool,
      backgroundIntervalSecs: json['background_interval_secs'] as int,
      cacheSizeMb: json['cache_size_mb'] as int,
    );
  }

  Map<String, dynamic> toJson() {
    return {
      'max_memory_mb': maxMemoryMb,
      'max_cpu_percent': maxCpuPercent,
      'max_concurrent_ai_tasks': maxConcurrentAiTasks,
      'max_database_size_mb': maxDatabaseSizeMb,
      'adaptive_management': adaptiveManagement,
      'background_interval_secs': backgroundIntervalSecs,
      'cache_size_mb': cacheSizeMb,
    };
  }

  ResourceConfig copyWith({
    int? maxMemoryMb,
    double? maxCpuPercent,
    int? maxConcurrentAiTasks,
    int? maxDatabaseSizeMb,
    bool? adaptiveManagement,
    int? backgroundIntervalSecs,
    int? cacheSizeMb,
  }) {
    return ResourceConfig(
      maxMemoryMb: maxMemoryMb ?? this.maxMemoryMb,
      maxCpuPercent: maxCpuPercent ?? this.maxCpuPercent,
      maxConcurrentAiTasks: maxConcurrentAiTasks ?? this.maxConcurrentAiTasks,
      maxDatabaseSizeMb: maxDatabaseSizeMb ?? this.maxDatabaseSizeMb,
      adaptiveManagement: adaptiveManagement ?? this.adaptiveManagement,
      backgroundIntervalSecs: backgroundIntervalSecs ?? this.backgroundIntervalSecs,
      cacheSizeMb: cacheSizeMb ?? this.cacheSizeMb,
    );
  }
}
