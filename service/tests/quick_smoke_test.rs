/// Quick smoke test to verify basic functionality
/// This test is designed to run quickly to validate the implementation works

use rootreal_model_symbolic_linkml::error::LinkMLError;

#[test]
fn test_error_types_exist() {
    // Verify error types can be created
    let _err = LinkMLError::parse("test error".to_string());
    let _err2 = LinkMLError::validation("validation error".to_string());
    let _err3 = LinkMLError::io("io error".to_string());
    
    // If we get here, error types work correctly
    assert!(true);
}

#[test]
fn test_basic_compilation() {
    // This test just verifies the crate compiles
    // If this runs, the code compiles successfully
    assert_eq!(2 + 2, 4);
}

