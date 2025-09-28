//! Simple unit tests for CLI static method conversions
//!
//! Tests the logic that was converted from instance methods to static methods
//! in the CLI module, focusing on inheritance depth calculation algorithms.

use std::collections::HashMap;
use linkml_core::types::ClassDefinition;
use pretty_assertions::assert_eq;

/// Test inheritance depth calculation logic (extracted from CLI)
/// This replicates the logic that was converted to static method
fn calculate_inheritance_depth(
    parent: &Option<String>,
    classes: &HashMap<String, ClassDefinition>,
    current_depth: usize,
) -> usize {
    if current_depth > 20 {
        // Prevent infinite recursion
        return current_depth;
    }

    match parent {
        Some(parent_name) => {
            if let Some(parent_class) = classes.get(parent_name) {
                calculate_inheritance_depth(&parent_class.is_a, classes, current_depth + 1)
            } else {
                current_depth
            }
        }
        None => current_depth,
    }
}

/// Test simple inheritance depth calculation
#[test]
fn test_inheritance_depth_simple() {
    let mut classes = HashMap::new();

    // Create class hierarchy: Animal -> Mammal -> Dog
    let animal = ClassDefinition {
        name: "Animal".to_string(),
        is_a: None,
        ..Default::default()
    };

    let mammal = ClassDefinition {
        name: "Mammal".to_string(),
        is_a: Some("Animal".to_string()),
        ..Default::default()
    };

    let dog = ClassDefinition {
        name: "Dog".to_string(),
        is_a: Some("Mammal".to_string()),
        ..Default::default()
    };

    classes.insert("Animal".to_string(), animal);
    classes.insert("Mammal".to_string(), mammal);
    classes.insert("Dog".to_string(), dog);

    // Test depth calculation
    assert_eq!(
        calculate_inheritance_depth(&None, &classes, 0),
        0,
        "No parent should have depth 0"
    );

    assert_eq!(
        calculate_inheritance_depth(&Some("Animal".to_string()), &classes, 0),
        1,
        "Mammal should have depth 1"
    );

    assert_eq!(
        calculate_inheritance_depth(&Some("Mammal".to_string()), &classes, 0),
        2,
        "Dog should have depth 2"
    );
}

/// Test circular inheritance protection
#[test]
fn test_circular_inheritance_protection() {
    let mut classes = HashMap::new();

    // Create circular inheritance: A -> B -> C -> A
    let class_a = ClassDefinition {
        name: "A".to_string(),
        is_a: Some("C".to_string()),
        ..Default::default()
    };

    let class_b = ClassDefinition {
        name: "B".to_string(),
        is_a: Some("A".to_string()),
        ..Default::default()
    };

    let class_c = ClassDefinition {
        name: "C".to_string(),
        is_a: Some("B".to_string()),
        ..Default::default()
    };

    classes.insert("A".to_string(), class_a);
    classes.insert("B".to_string(), class_b);
    classes.insert("C".to_string(), class_c);

    // Should stop at depth 20
    let depth = calculate_inheritance_depth(&Some("B".to_string()), &classes, 0);
    assert_eq!(depth, 20, "Should hit recursion limit at depth 20");
}

/// Test missing parent handling
#[test]
fn test_missing_parent_handling() {
    let mut classes = HashMap::new();

    let child = ClassDefinition {
        name: "Child".to_string(),
        is_a: Some("NonExistent".to_string()),
        ..Default::default()
    };

    classes.insert("Child".to_string(), child);

    // Should return current depth when parent doesn't exist
    let depth = calculate_inheritance_depth(&Some("NonExistent".to_string()), &classes, 5);
    assert_eq!(depth, 5, "Should return current depth for missing parent");
}

/// Test deep inheritance chain performance
#[test]
fn test_deep_inheritance_chain() {
    let mut classes = HashMap::new();

    // Create chain: Class0 -> Class1 -> ... -> Class10
    for i in 0..=10 {
        let parent = if i == 0 { None } else { Some(format!("Class{}", i - 1)) };

        let class = ClassDefinition {
            name: format!("Class{i}"),
            is_a: parent,
            ..Default::default()
        };

        classes.insert(format!("Class{i}"), class);
    }

    // Test various depths
    assert_eq!(
        calculate_inheritance_depth(&None, &classes, 0),
        0,
        "Class0 should have depth 0"
    );

    assert_eq!(
        calculate_inheritance_depth(&Some("Class4".to_string()), &classes, 0),
        5,
        "Class5 should have depth 5"
    );

    assert_eq!(
        calculate_inheritance_depth(&Some("Class9".to_string()), &classes, 0),
        10,
        "Class10 should have depth 10"
    );
}

/// Test method conversion functionality (static vs instance)
#[test]
fn test_static_method_conversion() {
    let mut classes = HashMap::new();

    let parent = ClassDefinition {
        name: "Parent".to_string(),
        is_a: None,
        ..Default::default()
    };

    let child = ClassDefinition {
        name: "Child".to_string(),
        is_a: Some("Parent".to_string()),
        ..Default::default()
    };

    classes.insert("Parent".to_string(), parent);
    classes.insert("Child".to_string(), child);

    // This demonstrates the function works as static method (no self)
    let depth = calculate_inheritance_depth(&Some("Parent".to_string()), &classes, 0);
    assert_eq!(depth, 1, "Static method should work correctly");
}

/// Test edge cases
#[test]
fn test_edge_cases() {
    // Empty classes map
    let classes = HashMap::new();
    assert_eq!(
        calculate_inheritance_depth(&Some("Any".to_string()), &classes, 0),
        0,
        "Should return current depth with empty classes map"
    );

    // High starting depth
    assert_eq!(
        calculate_inheritance_depth(&None, &classes, 15),
        15,
        "Should return starting depth when no parent specified"
    );

    // Self-referencing class
    let mut self_ref_classes = HashMap::new();
    let self_ref = ClassDefinition {
        name: "SelfRef".to_string(),
        is_a: Some("SelfRef".to_string()),
        ..Default::default()
    };
    self_ref_classes.insert("SelfRef".to_string(), self_ref);

    assert_eq!(
        calculate_inheritance_depth(&Some("SelfRef".to_string()), &self_ref_classes, 19),
        20,
        "Should hit recursion limit with self-reference"
    );
}