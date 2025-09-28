//! Integration tests for custom validators

use linkml_core::types::SchemaDefinition;
use linkml_core::types::{ClassDefinition, SlotDefinition};
use linkml_service::validator::{
    ValidationEngine,
    validators::{CustomValidatorBuilder, helpers},
};
use serde_json::{Value, json};
#[tokio::test]
async fn test_custom_validator_registration() {
    // Create a simple schema
    let mut schema = SchemaDefinition::new("test_schema");

    let mut person_class = ClassDefinition::new("Person");
    person_class.slots = vec!["name".to_string(), "email".to_string()];
    schema.classes.insert("Person".to_string(), person_class);

    let mut name_slot = SlotDefinition::new("name");
    name_slot.range = Some("string".to_string());
    schema.slots.insert("name".to_string(), name_slot);

    let mut email_slot = SlotDefinition::new("email");
    email_slot.range = Some("string".to_string());
    schema.slots.insert("email".to_string(), email_slot);

    // Create validation engine
    let mut engine = ValidationEngine::new(&schema).expect("Test operation failed");

    // Create a custom email validator
    let email_validator = CustomValidatorBuilder::new("strict_email_validator")
        .description("Validates email addresses with strict rules")
        .for_slots(vec!["email".to_string()])
        .validate_with(|value, _slot, context| {
            let mut issues = Vec::new();

            if let Value::String(s) = value {
                // Simple email validation
                let email_regex =
                    regex::Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$")
                        .expect("Test operation failed");

                if !email_regex.is_match(s) {
                    let mut issue = linkml_service::validator::report::ValidationIssue::error(
                        format!("'{}' is not a valid email address", s),
                        context.path(),
                        "strict_email_validator",
                    );
                    issue.code = Some("INVALID_EMAIL".to_string());
                    issues.push(issue);
                }
            }

            issues
        })
        .build()
        .expect("Test operation failed");

    // Register the custom validator
    engine.add_custom_validator(Box::new(email_validator));

    // Test valid data
    let valid_data = json!({
        "name": "John Doe",
        "email": "john.doe@example.com"
    });

    let report = engine
        .validate_as_class(&valid_data, "Person", None)
        .await
        .expect("Test operation failed");
    assert!(report.valid);
    assert!(report.errors().collect::<Vec<_>>().is_empty());

    // Test invalid email
    let invalid_data = json!({
        "name": "Jane Doe",
        "email": "not-an-email"
    });

    let report = engine
        .validate_as_class(&invalid_data, "Person", None)
        .await
        .expect("Test operation failed");
    assert!(!report.valid);
    let errors: Vec<_> = report.errors().collect();
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("not a valid email"));
}

#[tokio::test]
async fn test_custom_format_validator() {
    // Create schema with phone number field
    let mut schema = SchemaDefinition::new("test_schema");

    let mut contact_class = ClassDefinition::new("Contact");
    contact_class.slots = vec!["phone".to_string()];
    schema.classes.insert("Contact".to_string(), contact_class);

    let mut phone_slot = SlotDefinition::new("phone");
    phone_slot.range = Some("string".to_string());
    schema.slots.insert("phone".to_string(), phone_slot);

    // Create validation engine
    let mut engine = ValidationEngine::new(&schema).expect("Test operation failed");

    // Create a US phone number validator using the helper
    let phone_validator = helpers::format_validator("us_phone_validator", "US phone number", |s| {
        // Remove common formatting characters
        let digits: String = s.chars().filter(|c| c.is_numeric()).collect();

        // US phone numbers have 10 digits
        digits.len() == 10
    })
    .expect("Test operation failed");

    engine.add_custom_validator(Box::new(phone_validator));

    // Test valid US phone numbers
    let valid_phones = vec![
        "(123) 456-7890",
        "123-456-7890",
        "1234567890",
        "123.456.7890",
    ];

    for phone in valid_phones {
        let data = json!({ "phone": phone });
        let report = engine
            .validate_as_class(&data, "Contact", None)
            .await
            .expect("Test operation failed");
        assert!(report.valid, "Phone '{}' should be valid", phone);
    }

    // Test invalid phone numbers
    let invalid_phones = vec![
        "123-456",      // Too short
        "12345678901",  // Too long
        "abc-def-ghij", // Letters
    ];

    for phone in invalid_phones {
        let data = json!({ "phone": phone });
        let report = engine
            .validate_as_class(&data, "Contact", None)
            .await
            .expect("Test operation failed");
        assert!(!report.valid, "Phone '{}' should be invalid", phone);
    }
}

#[tokio::test]
async fn test_cross_field_validator() {
    // Create schema with date range
    let mut schema = SchemaDefinition::new("test_schema");

    let mut event_class = ClassDefinition::new("Event");
    event_class.slots = vec!["start_date".to_string(), "end_date".to_string()];
    schema.classes.insert("Event".to_string(), event_class);

    let mut start_slot = SlotDefinition::new("start_date");
    start_slot.range = Some("date".to_string());
    schema.slots.insert("start_date".to_string(), start_slot);

    let mut end_slot = SlotDefinition::new("end_date");
    end_slot.range = Some("date".to_string());
    schema.slots.insert("end_date".to_string(), end_slot);

    // Create validation engine
    let mut engine = ValidationEngine::new(&schema).expect("Test operation failed");

    // Create a cross-field validator that ensures end_date >= start_date
    let date_range_validator = CustomValidatorBuilder::new("date_range_validator")
        .description("Ensures end date is after start date")
        .for_slots(vec!["end_date".to_string()])
        .validate_with(|_value, _slot, context| {
            let mut issues = Vec::new();

            // Get the parent object to access both dates
            if let Some(parent) = context.parent() {
                if let Some(obj) = parent.as_object() {
                    if let (Some(Value::String(start)), Some(Value::String(end))) =
                        (obj.get("start_date"), obj.get("end_date"))
                    {
                        // Simple date comparison (assumes YYYY-MM-DD format)
                        if end < start {
                            let mut issue =
                                linkml_service::validator::report::ValidationIssue::error(
                                    format!(
                                        "End date ({}) must be after start date ({})",
                                        end, start
                                    ),
                                    context.path(),
                                    "date_range_validator",
                                );
                            issue.code = Some("DATE_RANGE_INVALID".to_string());
                            issues.push(issue);
                        }
                    }
                }
            }

            issues
        })
        .build()
        .expect("Test operation failed");

    engine.add_custom_validator(Box::new(date_range_validator));

    // Test valid date range
    let valid_data = json!({
        "start_date": "2024-01-01",
        "end_date": "2024-12-31"
    });

    let report = engine
        .validate_as_class(&valid_data, "Event", None)
        .await
        .expect("Test operation failed");
    assert!(report.valid);

    // Test invalid date range
    let invalid_data = json!({
        "start_date": "2024-12-31",
        "end_date": "2024-01-01"
    });

    let report = engine
        .validate_as_class(&invalid_data, "Event", None)
        .await
        .expect("Test operation failed");
    assert!(!report.valid);
    let errors: Vec<_> = report.errors().collect();
    assert!(errors[0].message.contains("must be after start date"));
}

#[tokio::test]
async fn test_custom_validator_with_predicate() {
    // Create schema with different types of codes
    let mut schema = SchemaDefinition::new("test_schema");

    let mut product_class = ClassDefinition::new("Product");
    product_class.slots = vec!["sku".to_string(), "barcode".to_string()];
    schema.classes.insert("Product".to_string(), product_class);

    let mut sku_slot = SlotDefinition::new("sku");
    sku_slot.range = Some("string".to_string());
    sku_slot.pattern = Some("^SKU-".to_string()); // SKUs must start with SKU-
    schema.slots.insert("sku".to_string(), sku_slot);

    let mut barcode_slot = SlotDefinition::new("barcode");
    barcode_slot.range = Some("string".to_string());
    schema.slots.insert("barcode".to_string(), barcode_slot);

    // Create validation engine
    let mut engine = ValidationEngine::new(&schema).expect("Test operation failed");

    // Create a validator that only applies to slots with a pattern
    let pattern_length_validator = CustomValidatorBuilder::new("pattern_length_validator")
        .description("Ensures values matching patterns have minimum length")
        .when(|slot| slot.pattern.is_some())
        .validate_with(|value, slot, context| {
            let mut issues = Vec::new();

            if let Value::String(s) = value {
                // If slot has a pattern and value matches it, ensure minimum length
                if let Some(pattern) = &slot.pattern {
                    if let Ok(re) = regex::Regex::new(pattern) {
                        if re.is_match(s) && s.len() < 10 {
                            issues.push(linkml_service::validator::report::ValidationIssue::error(
                                format!(
                                    "Value '{}' matching pattern must be at least 10 characters",
                                    s
                                ),
                                context.path(),
                                "pattern_length_validator",
                            ));
                        }
                    }
                }
            }

            issues
        })
        .build()
        .expect("Test operation failed");

    engine.add_custom_validator(Box::new(pattern_length_validator));

    // Test valid SKU (has pattern and length >= 10)
    let valid_data = json!({
        "sku": "SKU-123456",
        "barcode": "123"  // No pattern requirement
    });

    let report = engine
        .validate_as_class(&valid_data, "Product", None)
        .await
        .expect("Test operation failed");
    assert!(report.valid);

    // Test invalid SKU (has pattern but too short)
    let invalid_data = json!({
        "sku": "SKU-123",  // Matches pattern but too short
        "barcode": "123"
    });

    let report = engine
        .validate_as_class(&invalid_data, "Product", None)
        .await
        .expect("Test operation failed");
    assert!(!report.valid);
    let errors: Vec<_> = report.errors().collect();
    assert!(errors[0].message.contains("at least 10 characters"));
}

#[test]
fn test_custom_validator_builder_errors() {
    // Test builder without validation function
    let result = CustomValidatorBuilder::new("test")
        .description("Test validator")
        .build();

    assert!(result.is_err());
}
