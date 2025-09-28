//! Trait implementations for the `RustGenerator`

use super::core::RustGenerator;
use super::traits::{AsyncGenerator, GeneratedOutput, GeneratorOptions, GeneratorResult};
use async_trait::async_trait;

use linkml_core::prelude::*;
use std::collections::HashMap;
use std::fmt::Write;

#[async_trait]
impl AsyncGenerator for RustGenerator {
    fn name(&self) -> &str {
        self.name()
    }

    fn description(&self) -> &str {
        self.description()
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec!["rs"]
    }

    async fn validate_schema(&self, schema: &SchemaDefinition) -> GeneratorResult<()> {
        // Basic schema validation
        if schema.classes.is_empty() && schema.enums.is_empty() {
            return Err(super::traits::GeneratorError::Validation(
                "Schema must contain at least one class or enum".to_string(),
            ));
        }

        // Validate class references
        for (class_name, class) in &schema.classes {
            // Check parent class exists
            if let Some(parent) = &class.is_a
                && !schema.classes.contains_key(parent)
            {
                return Err(super::traits::GeneratorError::Validation(format!(
                    "Class '{class_name}' references unknown parent '{parent}'"
                )));
            }

            // Check slot references
            if !class.slots.is_empty() {
                let slots = &class.slots;
                for slot_name in slots {
                    if !schema.slots.contains_key(slot_name) {
                        return Err(super::traits::GeneratorError::Validation(format!(
                            "Class '{class_name}' references unknown slot '{slot_name}'"
                        )));
                    }
                }
            }
        }

        Ok(())
    }

    async fn generate(
        &self,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
    ) -> GeneratorResult<Vec<GeneratedOutput>> {
        // Validate schema
        AsyncGenerator::validate_schema(self, schema).await?;

        let mut outputs = Vec::new();
        let indent = &options.indent;

        // Generate main module file
        let mut main_output = String::new();

        // File header
        main_output.push_str(&Self::generate_header(schema)?);

        // Generate validation error enum
        main_output.push_str(&Self::generate_validation_error()?);

        // Generate enums first
        for (enum_name, enum_def) in &schema.enums {
            let enum_code = Self::generate_enum_rust(enum_name, enum_def, options, indent)?;
            main_output.push_str(&enum_code);
        }

        // Generate classes
        for (class_name, class) in &schema.classes {
            let class_code = Self::generate_class_rust(class_name, class, schema, options, indent)?;
            main_output.push_str(&class_code);
        }

        // Create output
        let filename = format!(
            "{}.rs",
            if schema.name.is_empty() {
                "schema"
            } else {
                &schema.name
            }
            .to_lowercase()
            .replace('-', "_")
        );

        let mut metadata = HashMap::new();
        metadata.insert("generator".to_string(), self.name().to_string());
        metadata.insert("schema_name".to_string(), schema.name.clone());

        outputs.push(GeneratedOutput {
            content: main_output,
            filename,
            metadata,
        });

        // Generate tests if requested
        if options.generate_tests {
            let test_output = self.generate_tests(schema, options)?;
            outputs.push(test_output);
        }

        Ok(outputs)
    }
}

impl RustGenerator {
    /// Generate test code for the schema
    pub(super) fn generate_tests(
        &self,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
    ) -> GeneratorResult<GeneratedOutput> {
        let mut output = String::new();

        // Test file header
        writeln!(&mut output, "//! Tests for generated schema code")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "#[cfg(test)]").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "mod tests {{").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "    use super::*;").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        // Generate tests for each class
        for (class_name, class) in &schema.classes {
            Self::generate_class_tests(&mut output, class_name, class, schema, options)?;
        }

        writeln!(&mut output, "}}").map_err(Self::fmt_error_to_generator_error)?;

        let filename = format!(
            "{}_tests.rs",
            if schema.name.is_empty() {
                "schema"
            } else {
                &schema.name
            }
            .to_lowercase()
            .replace('-', "_")
        );

        let mut metadata = HashMap::new();
        metadata.insert("generator".to_string(), self.name().to_string());
        metadata.insert("schema_name".to_string(), schema.name.clone());
        metadata.insert("file_type".to_string(), "tests".to_string());

        Ok(GeneratedOutput {
            content: output,
            filename,
            metadata,
        })
    }

    /// Generate tests for a specific class
    fn generate_class_tests(
        output: &mut String,
        class_name: &str,
        _class: &ClassDefinition,
        _schema: &SchemaDefinition,
        _options: &GeneratorOptions,
    ) -> GeneratorResult<()> {
        let struct_name = super::base::BaseCodeFormatter::to_pascal_case(class_name);

        // Test creation
        writeln!(output, "    #[test]").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            output,
            "    fn test_{}_creation() {{",
            class_name.to_lowercase()
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "        let instance = {struct_name}::new();")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "        assert!(instance.validate().is_ok());")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "    }}").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;

        // Test serialization
        writeln!(output, "    #[test]").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            output,
            "    fn test_{}_serialization() {{",
            class_name.to_lowercase()
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "        let instance = {struct_name}::new();")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            output,
            "        let json = instance.to_json().expect(\"Serialization failed\");"
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "        let _deserialized: {struct_name} = {struct_name}::from_json(&json).expect(\"Deserialization failed\");")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "    }}").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;

        Ok(())
    }
}
