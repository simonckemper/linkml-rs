//! Debug test to check pattern parsing

use linkml_service::parser::Parser;
use serde_json::json;

#[test]
fn debug_pattern_parsing() {
    let schema_yaml = r#"
id: https://example.org/test
name: test_schema
description: Test schema for pattern validation with capture groups

classes:
  DateRecord:
    name: DateRecord
    description: A record with date patterns
    slots:
      - iso_date
      - us_date
      - custom_date

slots:
  iso_date:
    name: iso_date
    description: ISO format date
    range: string
    pattern: "^(?P<year>\\d{4})-(?P<month>\\d{2})-(?P<day>\\d{2})$"

  us_date:
    name: us_date
    description: US format date
    range: string
    pattern: "^(?P<month>\\d{2})/(?P<day>\\d{2})/(?P<year>\\d{4})$"

  custom_date:
    name: custom_date
    description: Custom date with named parts
    range: string
    pattern: "^(?P<dayname>Mon|Tue|Wed|Thu|Fri|Sat|Sun), (?P<day>\\d{1,2}) (?P<month>Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec) (?P<year>\\d{4})$"
"#;

    // Parse schema
    let parser = Parser::new();
    let schema = parser
        .parse(schema_yaml, "yaml")
        .expect("LinkML operation in test should succeed");

    // Debug: Check patterns in parsed schema
    eprintln!("DEBUG: Checking parsed schema slots:");
    for (slot_name, slot_def) in &schema.slots {
        eprintln!("  Slot '{}': pattern = {:?}", slot_name, slot_def.pattern);
    }

    // Check class slots
    if let Some(date_record) = schema.classes.get("DateRecord") {
        eprintln!(
            "
DEBUG: DateRecord class has slots: {:?}",
            date_record.slots
        );

        for slot_name in &date_record.slots {
            if let Some(slot_def) = schema.slots.get(slot_name) {
                eprintln!(
                    "  Slot '{}' from class: pattern = {:?}",
                    slot_name, slot_def.pattern
                );
            } else {
                eprintln!("  WARNING: Slot '{}' not found in schema.slots!", slot_name);
            }
        }
    }

    // Verify each slot has the right pattern
    assert!(
        schema
            .slots
            .get("iso_date")
            .expect("LinkML operation in test should succeed")
            .pattern
            .as_ref()
            .expect("LinkML operation in test should succeed")
            .contains("year")
    );
    assert!(
        schema
            .slots
            .get("us_date")
            .expect("LinkML operation in test should succeed")
            .pattern
            .as_ref()
            .expect("LinkML operation in test should succeed")
            .contains("month")
    );
    assert!(
        schema
            .slots
            .get("custom_date")
            .expect("LinkML operation in test should succeed")
            .pattern
            .as_ref()
            .expect("LinkML operation in test should succeed")
            .contains("dayname")
    );
}
