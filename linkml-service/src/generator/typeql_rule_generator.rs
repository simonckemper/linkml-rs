//! `TypeQL` rule generation from ``LinkML`` constraints and expressions
//!
//! This module generates `TypeQL` rules for validation, inference, and
//! computed attributes based on ``LinkML`` schema definitions.

use super::traits::{GeneratorError, GeneratorResult};
use super::typeql_expression_translator::{ExpressionTranslator, TranslationContext};
use linkml_core::prelude::*;
use serde_json::Value;
use std::fmt::Write;

/// Types of rules that can be generated
#[derive(Debug, Clone, PartialEq)]
pub enum RuleType {
    /// Validation rule that adds error attributes
    Validation,
    /// Inference rule that adds derived attributes
    Inference,
    /// Computation rule that calculates values
    Computation,
    /// Classification rule that adds type information
    Classification,
}

/// A generated `TypeQL` rule
#[derive(Debug, Clone)]
pub struct TypeQLRule {
    /// Rule name
    pub name: String,
    /// Rule type
    pub rule_type: RuleType,
    /// When clause patterns
    pub when_patterns: Vec<String>,
    /// Then clause patterns
    pub then_patterns: Vec<String>,
    /// Rule description
    pub description: Option<String>,
    /// Dependencies on other rules
    pub dependencies: Vec<String>,
}

impl TypeQLRule {
    /// Generate `TypeQL` string for this rule
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - String formatting fails
    /// - I/O operations fail during generation
    /// - Rule structure is invalid
    pub fn to_typeql(&self) -> GeneratorResult<String> {
        let mut output = String::new();

        // Add description as comment
        if let Some(desc) = &self.description {
            writeln!(&mut output, "# {desc}")
                .map_err(|e| GeneratorError::Io(std::io::Error::other(e)))?;
        }

        // Rule definition
        writeln!(&mut output, "rule {}:", self.name)
            .map_err(|e| GeneratorError::Io(std::io::Error::other(e)))?;
        writeln!(&mut output, "when {{")
            .map_err(|e| GeneratorError::Io(std::io::Error::other(e)))?;

        // When patterns
        for (i, pattern) in self.when_patterns.iter().enumerate() {
            write!(&mut output, "    {pattern}")
                .map_err(|e| GeneratorError::Io(std::io::Error::other(e)))?;
            if i < self.when_patterns.len() - 1 {
                writeln!(&mut output, ";")
                    .map_err(|e| GeneratorError::Io(std::io::Error::other(e)))?;
            } else {
                writeln!(&mut output).map_err(|e| GeneratorError::Io(std::io::Error::other(e)))?;
            }
        }

        writeln!(&mut output, "}} then {{")
            .map_err(|e| GeneratorError::Io(std::io::Error::other(e)))?;

        // Then patterns
        for (i, pattern) in self.then_patterns.iter().enumerate() {
            write!(&mut output, "    {pattern}")
                .map_err(|e| GeneratorError::Io(std::io::Error::other(e)))?;
            if i < self.then_patterns.len() - 1 {
                writeln!(&mut output, ";")
                    .map_err(|e| GeneratorError::Io(std::io::Error::other(e)))?;
            } else {
                writeln!(&mut output).map_err(|e| GeneratorError::Io(std::io::Error::other(e)))?;
            }
        }

        writeln!(&mut output, "}};").map_err(|e| GeneratorError::Io(std::io::Error::other(e)))?;

        Ok(output)
    }
}

/// Generates `TypeQL` rules from ``LinkML`` schemas
pub struct RuleGenerator {
    /// Expression translator
    expression_translator: ExpressionTranslator,
    /// Generated rules
    rules: Vec<TypeQLRule>,
    /// Rule name counter for uniqueness
    rule_counter: usize,
    /// Generator options
    options: super::traits::GeneratorOptions,
}

impl Default for RuleGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl RuleGenerator {
    /// Create a new rule generator
    #[must_use]
    pub fn new() -> Self {
        Self {
            expression_translator: ExpressionTranslator::new(),
            rules: Vec::new(),
            rule_counter: 0,
            options: super::traits::GeneratorOptions::default(),
        }
    }

    /// Create generator with options
    #[must_use]
    pub fn with_options(options: super::traits::GeneratorOptions) -> Self {
        let mut generator = Self::new();
        generator.options = options;
        generator
    }

    /// Generate a unique rule name
    fn generate_rule_name(&mut self, prefix: &str) -> String {
        self.rule_counter += 1;
        format!("{}-{}", prefix, self.rule_counter)
    }

    /// Generate rules for a class
    pub fn generate_class_rules(
        &mut self,
        class_name: &str,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> Vec<TypeQLRule> {
        let mut rules = Vec::new();

        // Generate validation rules for slots
        for slot_name in &class.slots {
            if let Some(slot) = schema
                .slots
                .get(slot_name)
                .or_else(|| class.slot_usage.get(slot_name))
            {
                // Required field validation
                if slot.required == Some(true)
                    && let Some(rule) = self.generate_required_rule(class_name, slot_name)
                {
                    rules.push(rule);
                }

                // Pattern validation
                if let Some(pattern) = &slot.pattern
                    && let Some(rule) = self.generate_pattern_rule(class_name, slot_name, pattern)
                {
                    rules.push(rule);
                }

                // Range validation
                if (slot.minimum_value.is_some() || slot.maximum_value.is_some())
                    && let Some(rule) = self.generate_range_rule(class_name, slot_name, slot)
                {
                    rules.push(rule);
                }

                // Expression-based computed attributes
                if let Some(expr_str) = &slot.equals_expression
                    && let Some(rule) =
                        self.generate_expression_rule(class_name, slot_name, expr_str)
                {
                    rules.push(rule);
                }
            }
        }

        // Generate conditional requirement rules
        if let Some(if_required) = &class.if_required {
            for (i, (slot_name, conditional)) in if_required.iter().enumerate() {
                if let Some(rule) =
                    self.generate_conditional_rule(class_name, slot_name, conditional, i)
                {
                    rules.push(rule);
                }
            }
        }

        // Generate rules from class-level rules
        for linkml_rule in &class.rules {
            if let Some(rule) = self.generate_from_linkml_rule(class_name, linkml_rule) {
                rules.push(rule);
            }
        }

        rules
    }

    /// Generate a required field validation rule
    fn generate_required_rule(&mut self, class_name: &str, slot_name: &str) -> Option<TypeQLRule> {
        let rule_name = self.generate_rule_name(&format!(
            "{}-requires-{}",
            class_name.to_lowercase(),
            slot_name.to_lowercase()
        ));

        let when_patterns = vec![
            format!("$x isa {}", class_name.to_lowercase()),
            format!("not {{ $x has {slot_name} $v; }}"),
        ];

        let then_patterns = vec![format!(
            "$x has validation-error \"Missing required field: {}\"",
            slot_name
        )];

        Some(TypeQLRule {
            name: rule_name,
            rule_type: RuleType::Validation,
            when_patterns,
            then_patterns,
            description: Some(format!("Validates that {slot_name} is required")),
            dependencies: vec![],
        })
    }

    /// Generate a pattern validation rule
    fn generate_pattern_rule(
        &mut self,
        class_name: &str,
        slot_name: &str,
        pattern: &str,
    ) -> Option<TypeQLRule> {
        // Note: Pattern validation is handled by attribute regex in TypeQL
        // This generates a rule for invalid patterns
        let rule_name = self.generate_rule_name(&format!(
            "{}-{}-pattern",
            class_name.to_lowercase(),
            slot_name.to_lowercase()
        ));

        let when_patterns = vec![
            format!("$x isa {}", class_name.to_lowercase()),
            format!("$x has {slot_name} $v"),
            format!("not {{ $v like \"{}\"; }}", pattern),
        ];

        let then_patterns = vec![format!(
            "$x has validation-error \"Field {} does not match pattern: {}\"",
            slot_name, pattern
        )];

        Some(TypeQLRule {
            name: rule_name,
            rule_type: RuleType::Validation,
            when_patterns,
            then_patterns,
            description: Some(format!("Validates {slot_name} matches pattern")),
            dependencies: vec![],
        })
    }

    /// Generate a range validation rule
    fn generate_range_rule(
        &mut self,
        class_name: &str,
        slot_name: &str,
        slot: &SlotDefinition,
    ) -> Option<TypeQLRule> {
        let rule_name = self.generate_rule_name(&format!(
            "{}-{}-range",
            class_name.to_lowercase(),
            slot_name.to_lowercase()
        ));

        let mut when_patterns = vec![
            format!("$x isa {}", class_name.to_lowercase()),
            format!("$x has {slot_name} $v"),
        ];

        // Add range conditions
        let mut range_desc = Vec::new();

        if let Some(min) = &slot.minimum_value
            && let Value::Number(n) = min
        {
            when_patterns.push(format!("$v < {n}"));
            range_desc.push(format!(">= {n}"));
        }

        if let Some(max) = &slot.maximum_value
            && let Value::Number(n) = max
        {
            when_patterns.push(format!("$v > {n}"));
            range_desc.push(format!("<= {n}"));
        }

        if range_desc.is_empty() {
            return None;
        }

        let then_patterns = vec![format!(
            "$x has validation-error \"Field {} must be {}\"",
            slot_name,
            range_desc.join(" and ")
        )];

        Some(TypeQLRule {
            name: rule_name,
            rule_type: RuleType::Validation,
            when_patterns,
            then_patterns,
            description: Some(format!("Validates {slot_name} range constraints")),
            dependencies: vec![],
        })
    }

    /// Generate a rule from an expression
    fn generate_expression_rule(
        &mut self,
        class_name: &str,
        slot_name: &str,
        expr_str: &str,
    ) -> Option<TypeQLRule> {
        // Parse the expression
        let parser = crate::expression::parser::Parser::new();
        match parser.parse(expr_str) {
            Ok(expr) => {
                let entity_var = "$x".to_string();
                let mut ctx = TranslationContext::new(entity_var.clone());

                // Try to translate the expression
                match self.expression_translator.translate(&expr, &mut ctx) {
                    Ok(trans) => {
                        let rule_name = self.generate_rule_name(&format!(
                            "{}-compute-{}",
                            class_name.to_lowercase(),
                            slot_name.to_lowercase()
                        ));

                        let mut when_patterns =
                            vec![format!("$x isa {}", class_name.to_lowercase())];
                        when_patterns.extend(trans.patterns);

                        let then_patterns = vec![format!("$x has {} {}", slot_name, trans.result)];

                        Some(TypeQLRule {
                            name: rule_name,
                            rule_type: RuleType::Computation,
                            when_patterns,
                            then_patterns,
                            description: Some(format!("Computes {slot_name} from expression")),
                            dependencies: vec![],
                        })
                    }
                    Err(_) => {
                        // Complex expressions may need special handling
                        None
                    }
                }
            }
            Err(_) => None,
        }
    }

    /// Generate a conditional requirement rule
    fn generate_conditional_rule(
        &mut self,
        class_name: &str,
        slot_name: &str,
        conditional: &ConditionalRequirement,
        index: usize,
    ) -> Option<TypeQLRule> {
        let rule_name = self.generate_rule_name(&format!(
            "{}-conditional-{}",
            class_name.to_lowercase(),
            index
        ));

        let mut when_patterns = vec![format!("$x isa {}", class_name.to_lowercase())];

        // Add condition patterns based on the slot condition
        when_patterns.push(format!("$x has {slot_name} $cond"));

        if let Some(condition) = &conditional.condition {
            // Add value conditions
            if let Some(value) = &condition.equals_string {
                when_patterns.push(format!("$cond = \"{value}\""));
            }

            // NOTE: equals_string_in is not available in current `LinkML` core types
            // This would handle multiple allowed values if the field existed
            // if let Some(values) = &condition.equals_string_in {
            //     let values_str = values.iter()
            //         .map(|v| format!("\"{}\"", v))
            //         .collect::<Vec<_>>()
            //         .join(", ");
            //     when_patterns.push(format!("$cond = {{{values_str}}}"));
            // }
        }

        // Add patterns for missing required fields
        if let Some(then_required) = &conditional.then_required {
            for required_slot in then_required {
                when_patterns.push(format!("not {{ $x has {required_slot} $v; }}"));
            }

            // Generate error message
            let required_fields = then_required.join(", ");
            let default_value = "set".to_string();
            let condition_value = conditional
                .condition
                .as_ref()
                .and_then(|c| c.equals_string.as_ref())
                .unwrap_or(&default_value);

            let then_patterns = vec![format!(
                "$x has validation-error \"When {} is {}, the following fields are required: {}\"",
                slot_name, condition_value, required_fields
            )];

            Some(TypeQLRule {
                name: rule_name,
                rule_type: RuleType::Validation,
                when_patterns,
                then_patterns,
                description: Some("Conditional requirement validation".to_string()),
                dependencies: vec![],
            })
        } else {
            // No then_required fields, no rule needed
            None
        }
    }

    /// Generate a rule from a ``LinkML`` rule definition
    fn generate_from_linkml_rule(&mut self, class_name: &str, rule: &Rule) -> Option<TypeQLRule> {
        // This would require more complex parsing of rule preconditions/postconditions
        // For now, we'll create a placeholder
        let rule_id = rule
            .title
            .as_ref()
            .or(rule.description.as_ref())
            .map_or_else(
                || "rule".to_string(),
                |s| s.chars().take(20).collect::<String>(),
            );

        let rule_name = self.generate_rule_name(&format!(
            "{}-{}",
            class_name.to_lowercase(),
            rule_id.to_lowercase().replace(' ', "-")
        ));

        Some(TypeQLRule {
            name: rule_name,
            rule_type: RuleType::Inference,
            when_patterns: vec![format!("$x isa {}", class_name.to_lowercase())],
            then_patterns: vec![format!(
                "# Rule implementation pending for: {}",
                rule.description
                    .as_ref()
                    .unwrap_or(rule.title.as_ref().unwrap_or(&"unspecified".to_string()))
            )],
            description: rule.description.clone(),
            dependencies: vec![],
        })
    }

    /// Get all generated rules
    #[must_use]
    pub fn get_rules(&self) -> &[TypeQLRule] {
        &self.rules
    }

    /// Generate `TypeQL` string for all rules
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - String formatting fails during generation
    /// - I/O operations fail
    /// - Rule generation fails for any individual rule
    pub fn generate_typeql(&self) -> GeneratorResult<String> {
        let mut output = String::new();

        if !self.rules.is_empty() {
            writeln!(
                &mut output,
                "
# Generated Rules
"
            )
            .map_err(|e| GeneratorError::Io(std::io::Error::other(e)))?;

            // Group rules by type
            let mut validation_rules = Vec::new();
            let mut inference_rules = Vec::new();
            let mut computation_rules = Vec::new();

            for rule in &self.rules {
                match rule.rule_type {
                    RuleType::Validation => validation_rules.push(rule),
                    RuleType::Inference | RuleType::Classification => inference_rules.push(rule),
                    RuleType::Computation => computation_rules.push(rule),
                }
            }

            // Output validation rules
            if !validation_rules.is_empty() {
                writeln!(
                    &mut output,
                    "## Validation Rules
"
                )
                .map_err(|e| GeneratorError::Io(std::io::Error::other(e)))?;
                for rule in validation_rules {
                    let typeql = rule.to_typeql()?;
                    writeln!(&mut output, "{typeql}")
                        .map_err(|e| GeneratorError::Io(std::io::Error::other(e)))?;
                }
            }

            // Output computation rules
            if !computation_rules.is_empty() {
                writeln!(
                    &mut output,
                    "## Computed Attributes
"
                )
                .map_err(|e| GeneratorError::Io(std::io::Error::other(e)))?;
                for rule in computation_rules {
                    let typeql = rule.to_typeql()?;
                    writeln!(&mut output, "{typeql}")
                        .map_err(|e| GeneratorError::Io(std::io::Error::other(e)))?;
                }
            }

            // Output inference rules
            if !inference_rules.is_empty() {
                writeln!(
                    &mut output,
                    "## Inference Rules
"
                )
                .map_err(|e| GeneratorError::Io(std::io::Error::other(e)))?;
                for rule in inference_rules {
                    let typeql = rule.to_typeql()?;
                    writeln!(&mut output, "{typeql}")
                        .map_err(|e| GeneratorError::Io(std::io::Error::other(e)))?;
                }
            }
        }

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::types::SlotDefinition;

    #[test]
    fn test_required_rule_generation() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut generator = RuleGenerator::new();
        let rule = generator
            .generate_required_rule("Person", "name")
            .expect("should generate required rule: {}");

        assert_eq!(rule.rule_type, RuleType::Validation);
        assert!(rule.when_patterns.contains(&"$x isa person".to_string()));
        assert!(
            rule.when_patterns
                .contains(&"not { $x has name $v; }".to_string())
        );
        assert!(rule.then_patterns[0].contains("Missing required field: name"));
        Ok(())
    }

    #[test]
    fn test_range_rule_generation() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut generator = RuleGenerator::new();
        let mut slot = SlotDefinition::default();
        slot.minimum_value = Some(Value::Number(serde_json::Number::from(0)));
        slot.maximum_value = Some(Value::Number(serde_json::Number::from(150)));

        let rule = generator
            .generate_range_rule("Person", "age", &slot)
            .expect("should generate range rule: {}");

        assert_eq!(rule.rule_type, RuleType::Validation);
        assert!(rule.when_patterns.iter().any(|p| p.contains("$v < 0")));
        assert!(rule.when_patterns.iter().any(|p| p.contains("$v > 150")));
        Ok(())
    }

    #[test]
    fn test_rule_typeql_generation() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let rule = TypeQLRule {
            name: "test-rule".to_string(),
            rule_type: RuleType::Validation,
            when_patterns: vec![
                "$x isa person".to_string(),
                "not { $x has name $v; }".to_string(),
            ],
            then_patterns: vec!["$x has validation-error \"Missing name\"".to_string()],
            description: Some("Test rule".to_string()),
            dependencies: vec![],
        };

        let typeql = rule.to_typeql().expect("should generate TypeQL string: {}");

        assert!(typeql.contains("# Test rule"));
        assert!(typeql.contains("rule test-rule:"));
        assert!(typeql.contains("when {"));
        assert!(typeql.contains("$x isa person;"));
        assert!(typeql.contains("} then {"));
        assert!(typeql.contains("};"));
        Ok(())
    }
}
