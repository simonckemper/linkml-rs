//! Tests for SchemaView API

use linkml_core::types::SchemaDefinition;
use linkml_core::types::{ClassDefinition, SlotDefinition};
use linkml_service::schema_view::SchemaView;
use linkml_service::schema_view::analysis::SchemaAnalyzer;
use linkml_service::schema_view::navigation::ClassNavigator;
/// Create a test schema with inheritance
fn create_test_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition {
        id: "https://example.org/test".to_string(),
        name: "test_schema".to_string(),
        ..Default::default()
    };

    // Add base class
    let mut base_class = ClassDefinition::new("BaseClass");
    base_class.description = Some("Base class for testing".to_string());
    base_class.slots = vec!["id".to_string(), "name".to_string()];
    schema.classes.insert("BaseClass".to_string(), base_class);

    // Add derived class
    let mut derived_class = ClassDefinition::new("DerivedClass");
    derived_class.description = Some("Derived class for testing".to_string());
    derived_class.is_a = Some("BaseClass".to_string());
    derived_class.slots = vec!["extra_field".to_string()];
    schema
        .classes
        .insert("DerivedClass".to_string(), derived_class);

    // Add mixin class
    let mut mixin_class = ClassDefinition::new("TimestampMixin");
    mixin_class.mixin = Some(true);
    mixin_class.slots = vec!["created_at".to_string(), "updated_at".to_string()];
    schema
        .classes
        .insert("TimestampMixin".to_string(), mixin_class);

    // Add class with mixin
    let mut mixed_class = ClassDefinition::new("MixedClass");
    mixed_class.is_a = Some("BaseClass".to_string());
    mixed_class.mixins = vec!["TimestampMixin".to_string()];
    schema.classes.insert("MixedClass".to_string(), mixed_class);

    // Add slots
    let mut id_slot = SlotDefinition::new("id");
    id_slot.identifier = Some(true);
    id_slot.required = Some(true);
    id_slot.range = Some("string".to_string());
    schema.slots.insert("id".to_string(), id_slot);

    let mut name_slot = SlotDefinition::new("name");
    name_slot.required = Some(true);
    name_slot.range = Some("string".to_string());
    schema.slots.insert("name".to_string(), name_slot);

    let mut extra_slot = SlotDefinition::new("extra_field");
    extra_slot.range = Some("string".to_string());
    schema.slots.insert("extra_field".to_string(), extra_slot);

    let mut created_slot = SlotDefinition::new("created_at");
    created_slot.range = Some("datetime".to_string());
    schema.slots.insert("created_at".to_string(), created_slot);

    let mut updated_slot = SlotDefinition::new("updated_at");
    updated_slot.range = Some("datetime".to_string());
    schema.slots.insert("updated_at".to_string(), updated_slot);

    schema
}

#[test]
fn test_schema_view_creation() {
    let schema = create_test_schema();
    let view = SchemaView::new(schema).expect("Failed to create SchemaView");

    // Test that we can access classes
    let classes = view.all_classes().expect("Failed to get classes");
    assert_eq!(classes.len(), 4);
    assert!(classes.contains_key("BaseClass"));
    assert!(classes.contains_key("DerivedClass"));
}

#[test]
fn test_class_inheritance() {
    let schema = create_test_schema();
    let view = SchemaView::new(schema).expect("Failed to create SchemaView");

    // Test ancestors
    let ancestors = view
        .class_ancestors("DerivedClass")
        .expect("Failed to get ancestors");
    assert_eq!(ancestors, vec!["BaseClass".to_string()]);

    // Test descendants
    let descendants = view
        .class_descendants("BaseClass")
        .expect("Failed to get descendants");
    assert!(descendants.contains(&"DerivedClass".to_string()));
    assert!(descendants.contains(&"MixedClass".to_string()));
}

#[test]
fn test_induced_class() {
    let schema = create_test_schema();
    let view = SchemaView::new(schema).expect("Failed to create SchemaView");

    // Get induced class - should include inherited slots
    let induced = view
        .induced_class("DerivedClass")
        .expect("Failed to get induced class");

    // Should have both inherited and own slots
    assert!(induced.slots.contains(&"id".to_string()));
    assert!(induced.slots.contains(&"name".to_string()));
    assert!(induced.slots.contains(&"extra_field".to_string()));
}

#[test]
fn test_class_with_mixins() {
    let schema = create_test_schema();
    let view = SchemaView::new(schema).expect("Failed to create SchemaView");

    // Get induced class with mixins
    let induced = view
        .induced_class("MixedClass")
        .expect("Failed to get induced class");

    // Should have base slots and mixin slots
    assert!(induced.slots.contains(&"id".to_string()));
    assert!(induced.slots.contains(&"name".to_string()));
    assert!(induced.slots.contains(&"created_at".to_string()));
    assert!(induced.slots.contains(&"updated_at".to_string()));
}

#[test]
fn test_identifier_slot() {
    let schema = create_test_schema();
    let view = SchemaView::new(schema).expect("Failed to create SchemaView");

    // Test getting identifier slot
    let id_slot = view
        .get_identifier_slot("BaseClass")
        .expect("Failed to get identifier slot");
    assert_eq!(id_slot, Some("id".to_string()));

    // Mixin should not have identifier
    let mixin_id = view
        .get_identifier_slot("TimestampMixin")
        .expect("Failed to get identifier slot");
    assert_eq!(mixin_id, None);
}

#[test]
fn test_is_inlined() {
    let schema = create_test_schema();
    let view = SchemaView::new(schema).expect("Failed to create SchemaView");

    // Classes with identifiers should not be inlined
    let base_inlined = view
        .is_inlined("BaseClass")
        .expect("Failed to check inlined");
    assert!(!base_inlined);

    // Mixins without identifiers should be inlined
    let mixin_inlined = view
        .is_inlined("TimestampMixin")
        .expect("Failed to check inlined");
    assert!(mixin_inlined);
}

#[test]
fn test_class_navigator() {
    let schema = create_test_schema();
    let view = SchemaView::new(schema).expect("Failed to create SchemaView");
    let navigator = ClassNavigator::new(&view);

    // Test root classes
    let roots = navigator
        .get_root_classes()
        .expect("Failed to get root classes");
    assert!(roots.contains(&"BaseClass".to_string()));
    assert!(roots.contains(&"TimestampMixin".to_string()));

    // Test leaf classes
    let leaves = navigator
        .get_leaf_classes()
        .expect("Failed to get leaf classes");
    assert!(leaves.contains(&"DerivedClass".to_string()));
    assert!(leaves.contains(&"MixedClass".to_string()));

    // Test inheritance chain
    let chain = navigator
        .get_inheritance_chain("DerivedClass")
        .expect("Failed to get inheritance chain");
    assert_eq!(chain.start_class, "DerivedClass");
    assert_eq!(chain.chain, vec!["BaseClass"]);
    assert_eq!(chain.depth(), 1);
}

#[test]
fn test_schema_analyzer() {
    let schema = create_test_schema();
    let view = SchemaView::new(schema).expect("Failed to create SchemaView");
    let analyzer = SchemaAnalyzer::new(&view);

    // Test statistics
    let stats = analyzer
        .compute_statistics()
        .expect("Failed to compute statistics");
    assert_eq!(stats.class_count, 4);
    assert_eq!(stats.slot_count, 5);
    assert_eq!(stats.mixin_count, 1);
    assert_eq!(stats.root_class_count, 2);
    assert_eq!(stats.leaf_class_count, 3);

    // Test pattern search
    let pattern_results = analyzer
        .find_elements_by_pattern(".*Class")
        .expect("Failed to search by pattern");
    let classes = pattern_results.get("classes").expect("No classes found");
    assert!(classes.contains(&"BaseClass".to_string()));
    assert!(classes.contains(&"DerivedClass".to_string()));
    assert!(classes.contains(&"MixedClass".to_string()));
}

#[test]
fn test_usage_index() {
    let schema = create_test_schema();
    let view = SchemaView::new(schema).expect("Failed to create SchemaView");

    // Build usage index
    let usage_index = view.usage_index().expect("Failed to build usage index");

    // BaseClass should be used by DerivedClass and MixedClass
    let base_usage = usage_index
        .get_usage("BaseClass")
        .expect("No usage for BaseClass");
    assert_eq!(base_usage.used_by_classes.len(), 2);
    assert!(
        base_usage
            .used_by_classes
            .contains(&"DerivedClass".to_string())
    );
    assert!(
        base_usage
            .used_by_classes
            .contains(&"MixedClass".to_string())
    );

    // TimestampMixin should be marked as used as mixin
    let mixin_usage = usage_index
        .get_usage("TimestampMixin")
        .expect("No usage for TimestampMixin");
    assert!(mixin_usage.used_as_mixin);
}

#[test]
fn test_schema_view_caching() {
    let schema = create_test_schema();
    let view = SchemaView::new(schema).expect("Failed to create SchemaView");

    // First call should compute
    let induced1 = view
        .induced_class("DerivedClass")
        .expect("Failed to get induced class");

    // Second call should use cache (we can't directly test this, but it should be faster)
    let induced2 = view
        .induced_class("DerivedClass")
        .expect("Failed to get induced class");

    // Results should be identical
    assert_eq!(induced1.name, induced2.name);
    assert_eq!(induced1.slots, induced2.slots);
}
