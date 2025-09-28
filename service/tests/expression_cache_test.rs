//! Tests for expression result caching

use linkml_service::expression::{Evaluator, EvaluatorConfig, Parser};
use serde_json::json;
use std::collections::HashMap;
use std::time::Instant;

#[test]
fn test_expression_cache_hit() {
    let config = EvaluatorConfig {
        enable_cache: true,
        cache_size: 100,
        ..Default::default()
    };

    let evaluator = Evaluator::with_config(config);
    let parser = Parser::new();

    // Create a complex expression that takes some time to evaluate
    let expr = parser
        .parse("len(name) + max(10, 20) + min(5, 15)")
        .expect("Test operation failed");

    let mut context = HashMap::new();
    context.insert("name".to_string(), json!("test string"));

    // First evaluation - should compute
    let start = Instant::now();
    let result1 = evaluator
        .evaluate(&expr, &context)
        .expect("Test operation failed");
    let first_duration = start.elapsed();

    // Second evaluation - should use cache
    let start = Instant::now();
    let result2 = evaluator
        .evaluate(&expr, &context)
        .expect("Test operation failed");
    let second_duration = start.elapsed();

    // Results should be identical
    assert_eq!(result1, result2);
    assert_eq!(result1, json!(36.0)); // len("test string") + max(10,20) + min(5,15) = 11 + 20 + 5

    // Second evaluation should be significantly faster (at least 2x)
    // In practice, cache hit should be 100x+ faster, but we use a conservative check
    assert!(
        second_duration < first_duration / 2,
        "Cache hit should be faster: {:?} vs {:?}",
        second_duration,
        first_duration
    );
}

#[test]
fn test_expression_cache_miss_on_different_context() {
    let config = EvaluatorConfig {
        enable_cache: true,
        cache_size: 100,
        ..Default::default()
    };

    let evaluator = Evaluator::with_config(config);
    let parser = Parser::new();

    let expr = parser.parse("len(name)").expect("Test operation failed");

    // First context
    let mut context1 = HashMap::new();
    context1.insert("name".to_string(), json!("short"));

    let result1 = evaluator
        .evaluate(&expr, &context1)
        .expect("Test operation failed");
    assert_eq!(result1, json!(5));

    // Different context - should not use cache
    let mut context2 = HashMap::new();
    context2.insert("name".to_string(), json!("much longer string"));

    let result2 = evaluator
        .evaluate(&expr, &context2)
        .expect("Test operation failed");
    assert_eq!(result2, json!(18));

    // Verify cache is working by re-evaluating with first context
    let result3 = evaluator
        .evaluate(&expr, &context1)
        .expect("Test operation failed");
    assert_eq!(result3, json!(5));
}

#[test]
fn test_expression_cache_disabled() {
    let config = EvaluatorConfig {
        enable_cache: false,
        ..Default::default()
    };

    let evaluator = Evaluator::with_config(config);
    let parser = Parser::new();

    let expr = parser.parse("1 + 1").expect("Test operation failed");
    let context = HashMap::new();

    // Multiple evaluations should all compute (no caching)
    let result1 = evaluator
        .evaluate(&expr, &context)
        .expect("Test operation failed");
    let result2 = evaluator
        .evaluate(&expr, &context)
        .expect("Test operation failed");

    assert_eq!(result1, json!(2.0));
    assert_eq!(result2, json!(2.0));

    // Cache stats should be None when cache is disabled
    assert_eq!(evaluator.cache_stats(), None);
}

#[test]
fn test_expression_cache_clear() {
    let config = EvaluatorConfig {
        enable_cache: true,
        cache_size: 100,
        ..Default::default()
    };

    let evaluator = Evaluator::with_config(config);
    let parser = Parser::new();

    let expr = parser.parse("2 + 3").expect("Test operation failed");
    let context = HashMap::new();

    // Evaluate to populate cache
    let result1 = evaluator
        .evaluate(&expr, &context)
        .expect("Test operation failed");
    assert_eq!(result1, json!(5.0));

    // Check cache has entries
    if let Some((size, _capacity)) = evaluator.cache_stats() {
        assert_eq!(size, 1);
    }

    // Clear cache
    evaluator.clear_cache();

    // Check cache is empty
    if let Some((size, _capacity)) = evaluator.cache_stats() {
        assert_eq!(size, 0);
    }
}

#[test]
fn test_expression_cache_lru_eviction() {
    let config = EvaluatorConfig {
        enable_cache: true,
        cache_size: 3, // Small cache to test eviction
        ..Default::default()
    };

    let evaluator = Evaluator::with_config(config);
    let parser = Parser::new();

    // Evaluate 4 different expressions (more than cache size)
    for i in 0..4 {
        let expr = parser
            .parse(&format!("{} + 1", i))
            .expect("Test operation failed");
        let result = evaluator
            .evaluate(&expr, &HashMap::new())
            .expect("Test operation failed");
        assert_eq!(result, json!((i + 1) as f64));
    }

    // Cache should only have 3 entries (oldest was evicted)
    if let Some((size, capacity)) = evaluator.cache_stats() {
        assert_eq!(size, 3);
        assert_eq!(capacity, 3);
    }
}

#[test]
fn test_expression_cache_with_functions() {
    let config = EvaluatorConfig {
        enable_cache: true,
        cache_size: 100,
        ..Default::default()
    };

    let evaluator = Evaluator::with_config(config);
    let parser = Parser::new();

    // Expression with function calls
    let expr = parser
        .parse("max(len(str1), len(str2))")
        .expect("Test operation failed");

    let mut context = HashMap::new();
    context.insert("str1".to_string(), json!("hello"));
    context.insert("str2".to_string(), json!("world!!!"));

    // First evaluation
    let start = Instant::now();
    let result1 = evaluator
        .evaluate(&expr, &context)
        .expect("Test operation failed");
    let first_duration = start.elapsed();

    // Second evaluation - should use cache
    let start = Instant::now();
    let result2 = evaluator
        .evaluate(&expr, &context)
        .expect("Test operation failed");
    let second_duration = start.elapsed();

    assert_eq!(result1, json!(8.0)); // max(5, 8) = 8
    assert_eq!(result2, json!(8.0));

    // Cache hit should be faster
    assert!(second_duration < first_duration);
}

#[test]
fn test_expression_cache_complex_context() {
    let config = EvaluatorConfig {
        enable_cache: true,
        cache_size: 100,
        ..Default::default()
    };

    let evaluator = Evaluator::with_config(config);
    let parser = Parser::new();

    let expr = parser
        .parse("data.count * data.price")
        .expect("Test operation failed");

    // Complex nested context
    let mut context = HashMap::new();
    context.insert(
        "data".to_string(),
        json!({
            "count": 5,
            "price": 12.50
        }),
    );

    // Evaluate twice
    let result1 = evaluator
        .evaluate(&expr, &context)
        .expect("Test operation failed");
    let result2 = evaluator
        .evaluate(&expr, &context)
        .expect("Test operation failed");

    assert_eq!(result1, json!(62.5));
    assert_eq!(result2, json!(62.5));

    // Slightly different context should not use cache
    context.insert(
        "data".to_string(),
        json!({
            "count": 5,
            "price": 12.51  // Slightly different
        }),
    );

    let result3 = evaluator
        .evaluate(&expr, &context)
        .expect("Test operation failed");
    assert_eq!(result3, json!(62.55));
}
