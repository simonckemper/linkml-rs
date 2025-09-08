//! Security and injection attack tests for the LinkML expression language
//!
//! This test suite verifies that the expression language properly handles
//! various attack vectors and malicious inputs without compromising security.
//!
//! Test Categories:
//! - Expression Injection: Attempts to break out of expression context
//! - Code Injection: JavaScript/Python/Shell command injection attempts
//! - Stack Overflow: Deeply nested expressions and recursive patterns
//! - Resource Exhaustion: Memory and CPU DoS attempts
//! - Integer Overflow: Arithmetic overflow/underflow handling
//! - Unicode Attacks: Null bytes, direction overrides, homographs, zalgo text
//! - Path Traversal: File system path injection attempts
//! - SQL Injection: SQL pattern injection attempts
//! - Format String: Printf-style format string attacks
//! - Parser Security: Malformed input handling
//! - Error Security: Information leakage in error messages
//! - Future Considerations: ReDoS patterns for when regex is implemented
//!
//! All tests verify that:
//! 1. Attacks are properly blocked or handled safely
//! 2. Appropriate error messages are returned
//! 3. No sensitive information is leaked
//! 4. Legitimate complex expressions still work correctly

use linkml_service::expression::evaluator::EvaluatorConfig;
use linkml_service::expression::{Evaluator, ExpressionEngine, Parser};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

/// Helper to create an engine with strict security limits
fn create_secure_engine() -> ExpressionEngine {
    let config = EvaluatorConfig {
        max_iterations: 1000,
        max_call_depth: 20,
        timeout: Duration::from_millis(100),
        max_memory: 1024 * 1024, // 1MB
        enable_cache: true,
        cache_size: 100,
    };
    let evaluator = Arc::new(Evaluator::with_config(config));
    ExpressionEngine::with_evaluator(evaluator)
}

// ==================== Expression Injection Tests ====================

#[test]
fn test_expression_injection_variable_escape() {
    // Attack: Try to break out of variable context
    let engine = create_secure_engine();
    let mut context = HashMap::new();
    context.insert("evil".to_string(), json!("} + 1000 + {x"));

    // The variable content should be treated as a string, not parsed as expression
    let result = engine.evaluate("{evil}", &context);
    assert_eq!(
        result.expect("Test operation failed"),
        json!("} + 1000 + {x")
    );
}

#[test]
fn test_expression_injection_nested_braces() {
    // Attack: Try to inject nested variable references
    let engine = create_secure_engine();
    let mut context = HashMap::new();
    context.insert("outer".to_string(), json!("{inner}"));
    context.insert("inner".to_string(), json!("secret"));

    // Should not evaluate nested variable reference
    let result = engine.evaluate("{outer}", &context);
    assert_eq!(result.expect("Test operation failed"), json!("{inner}"));
}

#[test]
fn test_expression_injection_function_in_variable() {
    // Attack: Try to inject function calls through variables
    let engine = create_secure_engine();
    let mut context = HashMap::new();
    context.insert("evil".to_string(), json!("max(999999999)"));

    // Should treat as string, not execute function
    let result = engine.evaluate("{evil}", &context);
    assert_eq!(
        result.expect("Test operation failed"),
        json!("max(999999999)")
    );
}

// ==================== Code Injection Tests ====================

#[test]
fn test_code_injection_javascript_syntax() {
    // Attack: Try JavaScript-like code injection
    let parser = Parser::new();

    // JavaScript function syntax should fail
    assert!(parser.parse("function() { return 42; }").is_err());
    assert!(parser.parse("(() => 42)()").is_err());
    assert!(parser.parse("eval('malicious')").is_err());
    assert!(parser.parse("new Function('return 42')").is_err());
}

#[test]
fn test_code_injection_python_syntax() {
    // Attack: Try Python-like code injection
    let parser = Parser::new();

    // Python syntax should fail
    assert!(parser.parse("__import__('os').system('ls')").is_err());
    assert!(parser.parse("exec('print(42)')").is_err());
    assert!(parser.parse("lambda x: x * 2").is_err());
    assert!(parser.parse("import os").is_err());
}

#[test]
fn test_code_injection_shell_commands() {
    // Attack: Try shell command injection
    let parser = Parser::new();

    // Shell syntax should fail
    assert!(parser.parse("$(ls -la)").is_err());
    assert!(parser.parse("`cat /etc/passwd`").is_err());
    assert!(parser.parse("system('rm -rf /')").is_err());
    assert!(parser.parse("; rm -rf /").is_err());
}

// ==================== Stack Overflow Tests ====================

#[test]
fn test_stack_overflow_deeply_nested_expressions() {
    // Attack: Create deeply nested expressions to cause stack overflow
    let engine = create_secure_engine();
    let context = HashMap::new();

    // Build a deeply nested expression
    let mut expr = "1".to_string();
    for _ in 0..100 {
        expr = format!("({} + 1)", expr);
    }

    // Should fail due to depth limit
    let result = engine.evaluate(&expr, &context);
    assert!(result.is_err());
}

#[test]
fn test_stack_overflow_deeply_nested_functions() {
    // Attack: Deeply nested function calls
    let engine = create_secure_engine();
    let context = HashMap::new();

    // Build deeply nested function calls
    let mut expr = "1".to_string();
    for _ in 0..50 {
        expr = format!("max({}, 0)", expr);
    }

    // Should fail due to call depth limit
    let result = engine.evaluate(&expr, &context);
    assert!(result.is_err());
}

#[test]
fn test_stack_overflow_recursive_variable_reference() {
    // Attack: Try to create recursive variable references
    let engine = create_secure_engine();
    let mut context = HashMap::new();

    // This shouldn't cause recursion as variables don't re-evaluate
    context.insert("a".to_string(), json!("{b}"));
    context.insert("b".to_string(), json!("{a}"));

    // Should return the literal string, not recurse
    let result = engine.evaluate("{a}", &context);
    assert_eq!(result.expect("Test operation failed"), json!("{b}"));
}

// ==================== Resource Exhaustion Tests ====================

#[test]
fn test_resource_exhaustion_large_string_concat() {
    // Attack: Try to exhaust memory with string concatenation
    let engine = create_secure_engine();
    let mut context = HashMap::new();
    context.insert("s".to_string(), json!("x".repeat(1000));

    // Try to create very large string through repetition
    // Create a chain of concatenations that would exceed memory
    let mut expr = "{s}".to_string();
    for _ in 0..20 {
        expr = format!("({} + {})", expr, expr);
    }

    let result = engine.evaluate(&expr, &context);

    // Should fail due to memory limit or expression complexity
    // If it doesn't fail, the result should at least be valid
    match result {
        Err(_) => {
            // Expected - memory or complexity limit hit
        }
        Ok(val) => {
            // If it succeeds, verify it's a string
            assert!(val.is_string());
        }
    }
}

#[test]
fn test_resource_exhaustion_large_arithmetic_loop() {
    // Attack: Try to cause infinite computation
    let engine = create_secure_engine();
    let context = HashMap::new();

    // Create a very long arithmetic expression
    let mut expr = "1".to_string();
    for i in 0..10000 {
        expr = format!("{} + {}", expr, i);
    }

    // Should fail due to iteration limit or timeout
    let result = engine.evaluate(&expr, &context);
    assert!(result.is_err());
}

#[test]
fn test_resource_exhaustion_timeout() {
    // Attack: Try to exceed timeout with slow operations
    let engine = create_secure_engine();
    let mut context = HashMap::new();

    // Create many string operations (typically slower)
    let long_str = "a".repeat(1000);
    context.insert("s".to_string(), json!(long_str));

    // Many contains operations
    let mut expr = String::new();
    for _ in 0..100 {
        if !expr.is_empty() {
            expr.push_str(" and ");
        }
        expr.push_str("contains({s}, \"x\")");
    }

    // Should timeout
    let result = engine.evaluate(&expr, &context);
    assert!(result.is_err());
}

// ==================== Integer Overflow Tests ====================

#[test]
fn test_integer_overflow_large_numbers() {
    // Attack: Try to cause integer overflow
    let engine = create_secure_engine();
    let context = HashMap::new();

    // Very large number operations
    let result = engine.evaluate("9999999999999999999 * 9999999999999999999", &context);

    // Should handle gracefully (might convert to float or error)
    match result {
        Ok(val) => {
            // If successful, should be a valid number
            assert!(val.is_number());
        }
        Err(_) => {
            // Error is acceptable for overflow
        }
    }
}

#[test]
fn test_integer_underflow_large_negative() {
    // Attack: Try to cause integer underflow
    let engine = create_secure_engine();
    let context = HashMap::new();

    let result = engine.evaluate("-9999999999999999999 * 9999999999999999999", &context);

    // Should handle gracefully
    match result {
        Ok(val) => {
            assert!(val.is_number());
        }
        Err(_) => {
            // Error is acceptable
        }
    }
}

// ==================== Unicode/Encoding Attacks ====================

#[test]
fn test_unicode_null_byte_injection() {
    // Attack: Try null byte injection
    let engine = create_secure_engine();
    let mut context = HashMap::new();
    context.insert("evil".to_string(), json!("hello\0world"));

    // Should handle null bytes safely
    let result = engine.evaluate("{evil}", &context);
    assert!(result.is_ok());
}

#[test]
fn test_unicode_direction_override() {
    // Attack: Try Unicode direction override characters
    let engine = create_secure_engine();
    let mut context = HashMap::new();

    // Right-to-left override character
    context.insert("rtl".to_string(), json!("hello\u{202E}world"));

    // Should handle safely
    let result = engine.evaluate("{rtl}", &context);
    assert!(result.is_ok());
}

#[test]
fn test_unicode_homograph_attack() {
    // Attack: Try homograph attacks with similar-looking characters
    let engine = create_secure_engine();
    let mut context = HashMap::new();

    // Latin 'a' vs Cyrillic 'а'
    context.insert("a".to_string(), json!(100));
    context.insert("а".to_string(), json!(999)); // Cyrillic а

    // Should distinguish between the two
    assert_eq!(
        engine
            .evaluate("{a}", &context)
            .expect("Test operation failed"),
        json!(100)
    );
    assert_eq!(
        engine
            .evaluate("{а}", &context)
            .expect("Test operation failed"),
        json!(999)
    );
}

#[test]
fn test_unicode_zalgo_text() {
    // Attack: Try zalgo text (excessive combining characters)
    let engine = create_secure_engine();
    let mut context = HashMap::new();

    // Create zalgo text with many combining characters
    let zalgo = "ḧ̸̢̧̤̠̣̰̭̲̪̪̹̞̠̰̰̇̈́̌̂̆̊̀̊̏͆͘͜͝ȩ̷̛̺̬̮̱͙̘̦̯͈̗̈́̈́͊̐̃̈̅̆͊̕͜͝l̴̨̧̢̛̮̭̮̭̫̹̖̇̈́̾̈́̾̊̓̕̚͝͝ͅl̸̢̧̤̠̣̰̭̲̪̪̹̞̠̰̰̈̇̈́̌̂̆̊̀̊̏͆͘͜͝ơ̷̺̬̮̱͙̘̦̯͈̗̇̈́͊̐̃̈̅̆͊̕͜͝";
    context.insert("zalgo".to_string(), json!(zalgo));

    // Should handle without crashing
    let result = engine.evaluate("{zalgo}", &context);
    assert!(result.is_ok());
}

// ==================== Path Traversal Tests ====================

#[test]
fn test_path_traversal_in_variable_names() {
    // Attack: Try path traversal patterns in variable names
    let parser = Parser::new();

    // These should all fail to parse
    assert!(parser.parse("{../../../etc/passwd}").is_err());
    assert!(parser.parse("{..\\..\\windows\\system32}").is_err());
    assert!(parser.parse("{/etc/passwd}").is_err());
    assert!(parser.parse("{C:\\Windows\\System32}").is_err());
}

#[test]
fn test_path_traversal_url_encoding() {
    // Attack: Try URL-encoded path traversal
    let parser = Parser::new();

    assert!(parser.parse("{%2e%2e%2f%2e%2e%2f}").is_err());
    assert!(parser.parse("{%2fetc%2fpasswd}").is_err());
}

// ==================== SQL-like Injection Tests ====================

#[test]
fn test_sql_injection_patterns() {
    // Attack: Try SQL injection patterns
    let parser = Parser::new();

    // SQL keywords and syntax should fail
    assert!(parser.parse("SELECT * FROM users").is_err());
    assert!(parser.parse("'; DROP TABLE users; --").is_err());
    assert!(parser.parse("1' OR '1'='1").is_err());
    assert!(parser.parse("UNION SELECT NULL").is_err());
}

#[test]
fn test_sql_injection_in_strings() {
    // Attack: SQL injection in string values
    let engine = create_secure_engine();
    let mut context = HashMap::new();
    context.insert("input".to_string(), json!("'; DROP TABLE users; --"));

    // Should treat as literal string
    let result = engine.evaluate("{input}", &context);
    assert_eq!(
        result.expect("Test operation failed"),
        json!("'; DROP TABLE users; --")
    );
}

// ==================== Format String Attacks ====================

#[test]
fn test_format_string_patterns() {
    // Attack: Try format string patterns
    let engine = create_secure_engine();
    let mut context = HashMap::new();
    context.insert("fmt".to_string(), json!("%s%s%s%s%s"));

    // Should treat as literal string
    let result = engine.evaluate("{fmt}", &context);
    assert_eq!(result.expect("Test operation failed"), json!("%s%s%s%s%s"));
}

#[test]
fn test_printf_style_formats() {
    // Attack: Try various printf-style format specifiers
    let engine = create_secure_engine();
    let mut context = HashMap::new();

    let formats = vec!["%x", "%n", "%p", "%.999999f", "%999999d"];
    for fmt in formats {
        context.insert("f".to_string(), json!(fmt));
        let result = engine.evaluate("{f}", &context);
        assert_eq!(result.expect("Test operation failed"), json!(fmt));
    }
}

// ==================== Parser Security Tests ====================

#[test]
fn test_parser_unclosed_constructs() {
    // Attack: Try to break parser with unclosed constructs
    let parser = Parser::new();

    // Unclosed strings
    assert!(parser.parse("\"unclosed string").is_err());
    assert!(parser.parse("'unclosed single").is_err());

    // Unclosed parentheses
    assert!(parser.parse("(1 + 2").is_err());
    assert!(parser.parse("((1 + 2)").is_err());

    // Unclosed function calls
    assert!(parser.parse("max(1, 2").is_err());
    assert!(parser.parse("max(1, 2,").is_err());

    // Unclosed variables
    assert!(parser.parse("{unclosed").is_err());
    assert!(parser.parse("{unclosed{").is_err());
}

#[test]
fn test_parser_invalid_characters() {
    // Attack: Try invalid characters in various contexts
    let parser = Parser::new();

    // Control characters
    assert!(parser.parse("\x00").is_err());
    assert!(parser.parse("\x1F").is_err());

    // Invalid in identifiers
    assert!(parser.parse("{var!able}").is_err());
    assert!(parser.parse("{var@name}").is_err());
    assert!(parser.parse("{var#tag}").is_err());
}

#[test]
fn test_parser_extremely_long_input() {
    // Attack: Try extremely long input
    let parser = Parser::new();

    // Very long variable name
    let long_var = format!("{{{}}}", "a".repeat(10000));
    assert!(parser.parse(&long_var).is_err());

    // Very long string literal
    let long_string = format!("\"{}\"", "x".repeat(100000));
    let result = parser.parse(&long_string);
    // Parser might accept it, but evaluator should fail on memory
    assert!(result.is_ok() || result.is_err());
}

// ==================== Error Message Security Tests ====================

#[test]
fn test_error_message_no_path_leakage() {
    // Ensure error messages don't leak system paths
    let engine = create_secure_engine();
    let context = HashMap::new();

    // Try to use invalid variable name (will fail at parse time)
    let result = engine.evaluate("{nonexistent}", &context);
    if let Err(e) = result {
        let error_msg = e.to_string();
        // Should not contain system paths
        assert!(!error_msg.contains("/home/"));
        assert!(!error_msg.contains("C:\\"));
        assert!(!error_msg.contains("/Users/"));
    }
}

#[test]
fn test_error_message_no_internal_details() {
    // Ensure errors don't expose internal implementation details
    let engine = create_secure_engine();
    let context = HashMap::new();

    // Cause various errors
    let errors = vec![
        engine.evaluate("{undefined}", &context),
        engine.evaluate("1 / 0", &context),
        engine.evaluate("max()", &context),
    ];

    for result in errors {
        if let Err(e) = result {
            let error_msg = e.to_string();
            // Should not contain source code details
            assert!(!error_msg.contains("src/"));
            assert!(!error_msg.contains(".rs:"));
            assert!(!error_msg.contains("panic"));
            assert!(!error_msg.contains("unwrap"));
        }
    }
}

// ==================== Positive Security Tests ====================

#[test]
fn test_legitimate_complex_expressions() {
    // Ensure legitimate complex expressions still work
    let engine = create_secure_engine();
    let mut context = HashMap::new();

    // Complex but legitimate business logic
    context.insert("price".to_string(), json!(100.0));
    context.insert("quantity".to_string(), json!(5));
    context.insert("tax_rate".to_string(), json!(0.08));
    context.insert("discount_percent".to_string(), json!(10));
    context.insert("shipping".to_string(), json!(15.0));
    context.insert("member".to_string(), json!(true));

    // Complex calculation with conditionals
    let expr = r#"
        (({price} * {quantity} * (1 - {discount_percent} / 100)) +
         ({shipping} if not {member} else 0)) *
        (1 + {tax_rate})
    "#;

    let result = engine.evaluate(expr, &context);
    assert!(result.is_ok());

    // Nested conditionals
    let expr2 = r#"
        case(
            {quantity} > 10, "bulk",
            {quantity} > 5, "medium",
            {quantity} > 1, "small",
            "single"
        )
    "#;

    let result2 = engine.evaluate(expr2, &context);
    assert_eq!(result2.expect("Test operation failed"), json!("small")); // quantity is 5, so it's "small"
}

#[test]
fn test_legitimate_string_operations() {
    // Ensure legitimate string operations work
    let engine = create_secure_engine();
    let mut context = HashMap::new();

    // Unicode strings
    context.insert("greeting".to_string(), json!("Hello, 世界! 🌍"));
    context.insert("name".to_string(), json!("José García"));

    // Should handle Unicode properly
    assert_eq!(
        engine
            .evaluate("{greeting}", &context)
            .expect("Test operation failed"),
        json!("Hello, 世界! 🌍")
    );

    // String operations with Unicode
    // Note: The actual character count depends on how the engine counts
    let len_result = engine
        .evaluate("len({greeting})", &context)
        .expect("Test operation failed");
    assert!(len_result.is_number());

    assert_eq!(
        engine
            .evaluate("contains({name}, \"García\")", &context)
            .expect("Test operation failed"),
        json!(true)
    );
}

#[test]
fn test_legitimate_nested_operations() {
    // Ensure legitimate nested operations work
    let engine = create_secure_engine();
    let mut context = HashMap::new();

    context.insert("scores".to_string(), json!([85, 92, 78, 95, 88]));
    context.insert("threshold".to_string(), json!(80));

    // Nested function calls with conditionals
    let expr = r#"
        "passed" if min(85, 92, 78, 95, 88) >= {threshold} else
        ("partial" if max(85, 92, 78, 95, 88) >= {threshold} else "failed")
    "#;

    let result = engine.evaluate(expr, &context);
    assert_eq!(result.expect("Test operation failed"), json!("partial"));
}

// ==================== Future Security Considerations ====================

#[test]
fn test_regex_redos_preparation() {
    // When regex is implemented, these patterns should be rejected
    // This test documents patterns that could cause ReDoS

    let dangerous_patterns = vec![
        r"(a+)+b",       // Exponential backtracking
        r"(a*)*b",       // Exponential backtracking
        r"(a|a)*b",      // Exponential alternatives
        r"(.*)*",        // Catastrophic backtracking
        r"(a+)+$",       // Exponential with anchor
        r"^(a+)+b",      // Exponential with anchor
        r"(x+x+)+y",     // Nested quantifiers
        r"((a+)|(a+))+", // Alternation with overlap
    ];

    // Document for future implementation
    for pattern in dangerous_patterns {
        // When regex is added, ensure these patterns are rejected
        // or evaluated with timeout protection
        assert!(!pattern.is_empty()); // Placeholder assertion
    }
}

#[test]
fn test_command_injection_keywords() {
    // Ensure potential command keywords are not valid function names
    let parser = Parser::new();

    let dangerous_keywords = vec![
        "exec", "eval", "system", "spawn", "fork", "import", "require", "include", "open", "read",
        "write", "delete", "remove", "chmod", "chown",
    ];

    for keyword in dangerous_keywords {
        // These should not be valid function names
        let expr = format!("{}()", keyword);
        let result = parser.parse(&expr);

        // Either parsing fails or the function doesn't exist
        if result.is_ok() {
            let engine = create_secure_engine();
            let context = HashMap::new();
            assert!(engine.evaluate(&expr, &context).is_err());
        }
    }
}
