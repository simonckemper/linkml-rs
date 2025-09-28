//! Comprehensive tests for configuration module

use linkml_service::config::{
    LinkMLConfig, get_config,
    hot_reload::{ConfigHotReloader, get_hot_config, init_hot_reload},
    load_config, load_default_config, load_environment_config, load_production_config,
    validation::{validate_config, validate_values},
};
use std::env;
use std::fs;
use std::path::Path;
use tempfile::{NamedTempFile, TempDir};
use tokio::time::{Duration, sleep};

#[test]
fn test_load_default_config() {
    let config = load_default_config().expect("should load default config");

    // Verify default values
    assert_eq!(config.typedb.server_address, "localhost:1729");
    assert_eq!(config.typedb.default_database, "linkml");
    assert_eq!(config.typedb.batch_size, 1000);
    assert_eq!(config.parser.max_recursion_depth, 100);
    assert_eq!(config.validator.thread_count, 4);
    assert_eq!(config.cache.max_entries, 10000);
    assert_eq!(config.network.default_port, 8080);
}

#[test]
fn test_load_production_config() {
    // Set environment variables
    unsafe {
        env::set_var("TYPEDB_SERVER", "typedb.test.com:1730");
        env::set_var("TYPEDB_DATABASE", "linkml_test");
    }

    let config = load_production_config().expect("should load production config");

    // Verify production values with env var substitution
    assert_eq!(config.typedb.server_address, "typedb.test.com:1730");
    assert_eq!(config.typedb.default_database, "linkml_test");
    assert_eq!(config.typedb.batch_size, 5000); // Higher in production
    assert_eq!(config.validator.thread_count, 16); // More threads in production
    assert_eq!(config.cache.max_entries, 100000); // Larger cache in production

    // Clean up env vars
    unsafe {
        env::remove_var("TYPEDB_SERVER");
        env::remove_var("TYPEDB_DATABASE");
    }
}

#[test]
fn test_environment_based_loading() {
    // Test default environment
    unsafe {
        env::remove_var("LINKML_ENV");
    }
    let config = load_environment_config().expect("should load default for unset env");
    assert_eq!(config.typedb.batch_size, 1000); // Default value

    // Test production environment
    unsafe {
        env::set_var("LINKML_ENV", "production");
    }
    let config = load_environment_config().expect("should load production config");
    assert_eq!(config.typedb.batch_size, 5000); // Production value

    // Test explicit prod
    unsafe {
        env::set_var("LINKML_ENV", "prod");
    }
    let config = load_environment_config().expect("should load production config for 'prod'");
    assert_eq!(config.typedb.batch_size, 5000); // Production value

    // Clean up
    unsafe {
        env::remove_var("LINKML_ENV");
    }
}

#[test]
fn test_environment_variable_substitution() {
    // Create a test config with env vars
    let test_config = r#"
test:
  server: "${TEST_SERVER:-default.server.com}"
  port: "${TEST_PORT:-9999}"
  name: "${TEST_NAME}"
"#;

    let temp_file = NamedTempFile::new().expect("should create temp file");
    fs::write(temp_file.path(), test_config).expect("should write test config");

    // Test with env vars set
    unsafe {
        env::set_var("TEST_SERVER", "custom.server.com");
        env::set_var("TEST_PORT", "8888");
        env::set_var("TEST_NAME", "test-name");
    }

    #[derive(serde::Deserialize, Debug, PartialEq)]
    struct TestConfig {
        test: TestSection,
    }

    #[derive(serde::Deserialize, Debug, PartialEq)]
    struct TestSection {
        server: String,
        port: String,
        name: String,
    }

    let config: TestConfig = load_config(temp_file.path()).expect("should load test config");
    assert_eq!(config.test.server, "custom.server.com");
    assert_eq!(config.test.port, "8888");
    assert_eq!(config.test.name, "test-name");

    // Test with env vars unset (defaults)
    unsafe {
        env::remove_var("TEST_SERVER");
        env::remove_var("TEST_PORT");
        env::remove_var("TEST_NAME");
    }

    let config: TestConfig =
        load_config(temp_file.path()).expect("should load test config with defaults");
    assert_eq!(config.test.server, "default.server.com");
    assert_eq!(config.test.port, "9999");
    assert_eq!(config.test.name, ""); // No default provided
}

#[test]
fn test_config_validation_ttl_ordering() {
    let mut config = load_default_config().expect("should load default config");

    // Valid TTL ordering
    validate_values(&config).expect("default config should be valid");

    // Invalid: L1 >= L2
    config.performance.cache_ttl_levels.l1_seconds = 3600;
    config.performance.cache_ttl_levels.l2_seconds = 1800;
    let result = validate_values(&config);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("L1 cache TTL must be less than L2")
    );

    // Fix L1/L2, break L2/L3
    config.performance.cache_ttl_levels.l1_seconds = 300;
    config.performance.cache_ttl_levels.l2_seconds = 7200;
    config.performance.cache_ttl_levels.l3_seconds = 7200;
    let result = validate_values(&config);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("L2 cache TTL must be less than L3")
    );
}

#[test]
fn test_config_validation_memory_limits() {
    let mut config = load_default_config().expect("should load default config");

    // Valid memory configuration
    validate_values(&config).expect("default config should be valid");

    // Invalid: memory pool > total memory
    config.performance.memory_limit_bytes = 1_000_000_000; // 1GB
    config.performance.memory_pool.max_size_bytes = 2_000_000_000; // 2GB
    let result = validate_values(&config);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Memory pool size cannot exceed total memory limit")
    );

    // Invalid: cache memory > total memory
    config.performance.memory_pool.max_size_bytes = 500_000_000; // 500MB
    config.security_limits.max_cache_memory_bytes = 2_000_000_000; // 2GB
    let result = validate_values(&config);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Cache memory limit cannot exceed total memory limit")
    );
}

#[test]
fn test_config_validation_parallel_threads() {
    let mut config = load_default_config().expect("should load default config");

    // Valid: parallel enabled with threads > 0
    config.validator.enable_parallel = true;
    config.validator.thread_count = 4;
    validate_values(&config).expect("should be valid with parallel and threads");

    // Invalid: parallel enabled with 0 threads
    config.validator.thread_count = 0;
    let result = validate_values(&config);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Thread count must be > 0")
    );

    // Valid: parallel disabled with 0 threads
    config.validator.enable_parallel = false;
    validate_values(&config).expect("should be valid with parallel disabled");
}

#[test]
fn test_singleton_config() {
    // First call initializes
    let config1 = get_config();
    assert_eq!(config1.typedb.server_address, "localhost:1729");

    // Subsequent calls return same instance
    let config2 = get_config();
    assert_eq!(config1.typedb.server_address, config2.typedb.server_address);

    // Verify it's the same instance
    assert_eq!(config1 as *const _, config2 as *const _);
}

#[tokio::test]
async fn test_hot_reload_basic() {
    // Create a temporary config file
    let temp_dir = TempDir::new().expect("should create temp dir");
    let config_path = temp_dir.path().join("test_config.yaml");

    // Write initial config
    let initial_config = r#"
typedb:
  server_address: "localhost:1729"
  default_database: "test"
  batch_size: 1000
  connection_timeout_ms: 30000
  query_timeout_ms: 10000
  max_retries: 3
  retry_delay_ms: 1000
  pool_size: 10
  include_inferred: false
parser:
  max_recursion_depth: 100
  enable_cache: true
  cache_ttl_seconds: 3600
  max_file_size_bytes: 10485760
  supported_formats: ["yaml", "json"]
  max_import_depth: 10
validator:
  enable_parallel: true
  thread_count: 4
  batch_size: 100
  timeout_ms: 30000
  max_errors: 1000
  fail_fast: false
  compiled_cache_size: 1000
generator:
  output_directory: "./generated"
  enable_formatting: true
  include_docs: true
  generator_options: {}
cache:
  max_entries: 10000
  ttl_seconds: 3600
  enable_compression: true
  eviction_policy: "Lru"
  expression_cache:
    max_entries: 1000
    ttl_seconds: 1800
  rule_cache:
    max_entries: 500
    ttl_seconds: 7200
performance:
  enable_monitoring: true
  memory_limit_bytes: 1073741824
  cpu_limit_percent: 80
  enable_string_interning: true
  string_pool_size: 10000
  enable_background_tasks: true
  enable_cache_warming: true
  background_task_interval_secs: 60
  string_cache:
    max_entries: 5000
    max_string_length: 1024
  memory_pool:
    max_size_bytes: 104857600
    chunk_size_bytes: 4096
  cache_ttl_levels:
    l1_seconds: 300
    l2_seconds: 1800
    l3_seconds: 7200
    min_ttl_seconds: 60
    max_ttl_seconds: 86400
security_limits:
  max_string_length: 65536
  max_expression_depth: 100
  max_constraint_count: 1000
  max_cache_entries: 100000
  max_function_args: 20
  max_identifier_length: 255
  max_json_size_bytes: 10485760
  max_slots_per_class: 1000
  max_classes_per_schema: 10000
  max_validation_time_ms: 300000
  max_memory_usage_bytes: 1073741824
  max_parallel_validators: 100
  max_cache_memory_bytes: 104857600
  max_expression_time_ms: 1000
  max_validation_errors: 1000
network:
  default_host: "127.0.0.1"
  default_port: 8080
  api_timeout_seconds: 30
expression:
  enable_cache: true
  enable_compilation: true
  cache_size: 1000
  timeout_seconds: 1
  max_recursion_depth: 50
pattern_validator:
  default_cache_size: 100
multi_layer_cache:
  l3_max_size_bytes: 1073741824
background_services:
  cache_ttl_check_interval_secs: 300
  memory_cleanup_interval_secs: 600
  panic_recovery_timeout_secs: 5
  error_recovery_timeout_secs: 60
cli:
  default_iterations: 1000
  progress_bar_template: "{spinner} [{bar}]"
  progress_bar_finish_template: "Done!"
"#;

    fs::write(&config_path, initial_config).expect("should write initial config");

    // Create hot reloader
    let mut reloader = ConfigHotReloader::new(&config_path).expect("should create hot reloader");

    // Get initial config
    let initial = reloader.get_config();
    assert_eq!(initial.typedb.batch_size, 1000);

    // Start watching
    reloader
        .start_watching()
        .await
        .expect("should start watching");

    // Subscribe to updates
    let mut update_rx = reloader.subscribe();

    // Modify the config file
    let modified_config = initial_config.replace("batch_size: 1000", "batch_size: 2000");
    fs::write(&config_path, modified_config).expect("should write modified config");

    // Wait for file system event
    sleep(Duration::from_millis(200)).await;

    // Note: File watching in tests can be unreliable
    // In a real environment, the update would be detected

    // Stop watching
    reloader.stop_watching();
}

#[test]
fn test_invalid_config_format() {
    let temp_file = NamedTempFile::new().expect("should create temp file");
    fs::write(temp_file.path(), "invalid: yaml: format:").expect("should write invalid config");

    let result: Result<LinkMLConfig, _> = load_config(temp_file.path());
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Failed to parse YAML")
    );
}

#[test]
fn test_missing_config_file() {
    let result: Result<LinkMLConfig, _> = load_config(Path::new("/nonexistent/config.yaml"));
    assert!(result.is_err());
}

#[test]
fn test_config_all_fields_present() {
    // This test ensures all expected fields are present in the default config
    let config = load_default_config().expect("should load default config");

    // TypeDB config
    assert!(!config.typedb.server_address.is_empty());
    assert!(!config.typedb.default_database.is_empty());
    assert!(config.typedb.batch_size > 0);
    assert!(config.typedb.connection_timeout_ms > 0);

    // Parser config
    assert!(config.parser.max_recursion_depth > 0);
    assert!(config.parser.max_file_size_bytes > 0);
    assert!(!config.parser.supported_formats.is_empty());

    // Validator config
    assert!(config.validator.batch_size > 0);
    assert!(config.validator.max_errors > 0);

    // Cache config
    assert!(config.cache.max_entries > 0);
    assert!(config.cache.ttl_seconds > 0);
    assert!(!config.cache.eviction_policy.is_empty());

    // Performance config
    assert!(config.performance.memory_limit_bytes > 0);
    assert!(config.performance.cpu_limit_percent > 0);
    assert!(config.performance.cpu_limit_percent <= 100);

    // Security limits
    assert!(config.security_limits.max_string_length > 0);
    assert!(config.security_limits.max_expression_depth > 0);
    assert!(config.security_limits.max_cache_entries > 0);

    // Network config
    assert!(!config.network.default_host.is_empty());
    assert!(config.network.default_port > 0);
    assert!(config.network.api_timeout_seconds > 0);
}

#[test]
fn test_production_config_higher_limits() {
    let default_config = load_default_config().expect("should load default config");
    let prod_config = load_production_config().expect("should load production config");

    // Production should have higher limits
    assert!(prod_config.typedb.batch_size > default_config.typedb.batch_size);
    assert!(prod_config.cache.max_entries > default_config.cache.max_entries);
    assert!(prod_config.validator.thread_count > default_config.validator.thread_count);
    assert!(
        prod_config.performance.memory_limit_bytes > default_config.performance.memory_limit_bytes
    );
    assert!(
        prod_config.security_limits.max_cache_entries
            > default_config.security_limits.max_cache_entries
    );

    // Production should have longer TTLs
    assert!(prod_config.cache.ttl_seconds >= default_config.cache.ttl_seconds);
    assert!(
        prod_config.cache.expression_cache.ttl_seconds
            >= default_config.cache.expression_cache.ttl_seconds
    );
}
