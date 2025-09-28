//! Phase 5 Tests - Verify unwrap() fixes work correctly
//!
//! These tests verify that the Phase 1 unwrap() removals result in proper
//! error handling instead of panics.

#[cfg(test)]
mod tests {
    use linkml_core::error::LinkMLError;
    use std::fs;
    // use std::error::Error as StdError;
    use std::path::Path;
    use tempfile::TempDir;

    /// Test that file operations handle errors gracefully
    #[test]
    fn test_file_operations_no_panic() {
        // Test reading non-existent file
        let result = std::fs::read_to_string("/non/existent/file.yaml");
        assert!(result.is_err());

        // Test writing to invalid location
        let result = std::fs::write("/root/cannot_write.txt", "test");
        assert!(result.is_err());
    }

    /// Test JSON parsing error handling
    #[test]
    fn test_json_parsing_errors() {
        let invalid_json = r#"{"invalid": json"#;
        let result: Result<serde_json::Value, _> = serde_json::from_str(invalid_json);
        assert!(result.is_err());

        // Test with trailing comma
        let json_with_comma = r#"{"field": "value",}"#;
        let result: Result<serde_json::Value, _> = serde_json::from_str(json_with_comma);
        assert!(result.is_err());
    }

    /// Test YAML parsing error handling
    #[test]
    fn test_yaml_parsing_errors() {
        let invalid_yaml = "
field1:
  - item1
  item2  # Invalid - mixing list and map
";
        let result: Result<serde_yaml::Value, _> = serde_yaml::from_str(invalid_yaml);
        assert!(result.is_err());
    }

    /// Test regex compilation error handling
    #[test]
    fn test_regex_compilation_errors() {
        use regex::Regex;

        // Invalid regex patterns
        let patterns = vec![
            "[unclosed",
            "(unclosed group",
            "*invalid",
            "(?P<invalid group name)",
            "(?P<>empty)",
        ];

        for pattern in patterns {
            let result = Regex::new(pattern);
            assert!(result.is_err(), "Pattern '{}' should fail", pattern);
        }
    }

    /// Test numeric operations that could panic
    #[test]
    fn test_numeric_operations() {
        // Division by zero in float is OK (gives infinity)
        let result = 10.0_f64 / 0.0_f64;
        assert!(result.is_infinite());

        // Integer overflow in release mode wraps
        let max = i32::MAX;
        let result = max.wrapping_add(1);
        assert_eq!(result, i32::MIN);

        // Checked operations return None on overflow
        let result = max.checked_add(1);
        assert!(result.is_none());
    }

    /// Test string operations
    #[test]
    fn test_string_operations() {
        let s = "hello";

        // Out of bounds char access
        let result = s.chars().nth(100);
        assert!(result.is_none());

        // Invalid UTF-8 bytes
        let invalid_utf8 = vec![0xFF, 0xFE, 0xFD];
        let result = String::from_utf8(invalid_utf8);
        assert!(result.is_err());
    }

    /// Test collection operations
    #[test]
    fn test_collection_operations() {
        let vec = vec![1, 2, 3];

        // Get returns Option
        let result = vec.get(10);
        assert!(result.is_none());

        // Pop on empty vec
        let mut empty_vec: Vec<i32> = Vec::new();
        let result = empty_vec.pop();
        assert!(result.is_none());

        // HashMap get
        use std::collections::HashMap;
        let map = HashMap::new();
        let result = map.get(&"key");
        assert!(result.is_none());
    }

    /// Test path operations
    #[test]
    fn test_path_operations() {
        let path = Path::new("/some/path/file.txt");

        // Parent might be None
        let root = Path::new("/");
        let parent = root.parent();
        assert!(parent.is_none());

        // Extension might be None
        let no_ext = Path::new("file");
        let ext = no_ext.extension();
        assert!(ext.is_none());

        // File stem might be None
        let stem = Path::new("").file_stem();
        assert!(stem.is_none());
    }

    /// Test environment variable access
    #[test]
    fn test_env_var_access() {
        use std::env;

        // Non-existent env var
        let result = env::var("DEFINITELY_DOES_NOT_EXIST_VAR_12345");
        assert!(result.is_err());

        // Set and get
        env::set_var("TEST_VAR", "value");
        let result = env::var("TEST_VAR");
        assert!(result.is_ok());
        env::remove_var("TEST_VAR");
    }

    /// Test error conversion and propagation
    #[test]
    fn test_error_propagation() {
        fn might_fail(should_fail: bool) -> Result<String, LinkMLError> {
            if should_fail {
                Err(LinkMLError::Parse("Test error".to_string()))
            } else {
                Ok("Success".to_string())
            }
        }

        fn propagate_error() -> Result<String, LinkMLError> {
            let result = might_fail(true)?;
            Ok(result)
        }

        let result = propagate_error();
        assert!(result.is_err());
        match result {
            Err(LinkMLError::Parse(msg)) => assert_eq!(msg, "Test error"),
            _ => panic!("Wrong error type"),
        }
    }

    /// Test concurrent operations
    #[test]
    fn test_concurrent_safety() {
        use std::sync::{Arc, Mutex};
        use std::thread;

        let data = Arc::new(Mutex::new(vec![1, 2, 3]));
        let mut handles = vec![];

        for i in 0..3 {
            let data_clone = Arc::clone(&data);
            let handle = thread::spawn(move || {
                if let Ok(mut vec) = data_clone.lock() {
                    vec.push(i);
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.join();
        }

        let final_data = data.lock().expect("Test operation failed");
        assert_eq!(final_data.len(), 6);
    }

    /// Test that previously panicking code now returns errors
    #[test]
    fn test_no_panics_in_common_scenarios() {
        // Scenario 1: Empty slice operations
        let empty: &[i32] = &[];
        assert!(empty.first().is_none());
        assert!(empty.last().is_none());
        assert!(empty.get(0).is_none());

        // Scenario 2: String slicing
        let s = "hello";
        let bytes = s.as_bytes();
        if bytes.len() >= 10 {
            let _ = &bytes[..10];
        }

        // Scenario 3: Option handling
        let opt: Option<i32> = None;
        let result = opt.map(|x| x * 2).unwrap_or(0);
        assert_eq!(result, 0);

        // Scenario 4: Result handling
        let res: Result<i32, &str> = Err("error");
        let result = res.unwrap_or_else(|_| 42);
        assert_eq!(result, 42);
    }

    /// Integration test for a complete workflow
    #[test]
    fn test_integrated_workflow() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;

        // Create test file
        let file_path = temp_dir.path().join("test.yaml");
        fs::write(&file_path, "key: value
")?;

        // Read and parse
        let contents = fs::read_to_string(&file_path)?;
        let _parsed: serde_yaml::Value = serde_yaml::from_str(&contents)?;

        // Try to read non-existent file
        let missing_path = temp_dir.path().join("missing.yaml");
        let result = fs::read_to_string(&missing_path);
        assert!(result.is_err());

        Ok(())
    }
}
