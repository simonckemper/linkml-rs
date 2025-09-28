//! Rule engine for class-level validation
//!
//! This module implements if-then-else validation logic for LinkML classes,
//! enabling complex cross-field validation scenarios.

use linkml_core::types::SchemaDefinition;
pub mod cache;
pub mod evaluator;
pub mod executor;
pub mod inheritance;
pub mod matcher;
pub mod types;

use parking_lot::RwLock;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

use crate::expression::ExpressionEngine;
use crate::validator::context::ValidationContext;
use crate::validator::report::ValidationIssue;

pub use types::{CompiledRule, RuleExecutionContext, RuleExecutionStrategy};

/// Main rule engine for evaluating class-level rules
pub struct RuleEngine {
    /// Schema containing rule definitions
    schema: Arc<SchemaDefinition>,
    /// Expression engine for evaluating conditions
    expression_engine: Arc<ExpressionEngine>,
    /// Cache of compiled rules
    rule_cache: Arc<RwLock<HashMap<String, Vec<CompiledRule>>>>,
    /// Rule execution strategy
    execution_strategy: RuleExecutionStrategy,
}

impl RuleEngine {
    /// Create a new rule engine
    #[must_use]
    pub fn new(schema: Arc<SchemaDefinition>) -> Self {
        Self {
            schema,
            expression_engine: Arc::new(ExpressionEngine::new()),
            rule_cache: Arc::new(RwLock::new(HashMap::new())),
            execution_strategy: RuleExecutionStrategy::default(),
        }
    }

    /// Create a rule engine with a custom expression engine
    #[must_use]
    pub fn with_expression_engine(
        schema: Arc<SchemaDefinition>,
        expression_engine: Arc<ExpressionEngine>,
    ) -> Self {
        Self {
            schema,
            expression_engine,
            rule_cache: Arc::new(RwLock::new(HashMap::new())),
            execution_strategy: RuleExecutionStrategy::default(),
        }
    }

    /// Create a rule engine with a custom execution strategy
    #[must_use]
    pub fn with_strategy(schema: Arc<SchemaDefinition>, strategy: RuleExecutionStrategy) -> Self {
        Self {
            schema,
            expression_engine: Arc::new(ExpressionEngine::new()),
            rule_cache: Arc::new(RwLock::new(HashMap::new())),
            execution_strategy: strategy,
        }
    }

    /// Validate an instance against all applicable rules for its class
    pub fn validate(
        &self,
        instance: &Value,
        class_name: &str,
        context: &mut ValidationContext,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        // Get all applicable rules (including inherited)
        let rules = match self.get_applicable_rules(class_name) {
            Ok(rules) => rules,
            Err(e) => {
                issues.push(ValidationIssue::error(
                    format!("Failed to get rules for class {class_name}: {e}"),
                    context.path(),
                    "RuleEngine",
                ));
                return issues;
            }
        };

        // Create execution context
        let mut exec_context =
            RuleExecutionContext::new(instance.clone(), class_name.to_string(), context);

        // Execute rules based on strategy
        let executor = executor::RuleExecutor::new(self.expression_engine.clone());

        match executor.execute_rules(&rules, &mut exec_context, self.execution_strategy) {
            Ok(rule_issues) => issues.extend(rule_issues),
            Err(e) => {
                issues.push(ValidationIssue::error(
                    format!("Rule execution failed: {e}"),
                    context.path(),
                    "RuleEngine",
                ));
            }
        }

        issues
    }

    /// Get all applicable rules for a class (including inherited)
    fn get_applicable_rules(
        &self,
        class_name: &str,
    ) -> linkml_core::error::Result<Vec<CompiledRule>> {
        // Check cache first
        {
            let cache = self.rule_cache.read();
            if let Some(cached_rules) = cache.get(class_name) {
                return Ok(cached_rules.clone());
            }
        }

        // Compile rules for this class
        let compiled_rules = self.compile_class_rules(class_name)?;

        // Cache the compiled rules
        {
            let mut cache = self.rule_cache.write();
            cache.insert(class_name.to_string(), compiled_rules.clone());
        }

        Ok(compiled_rules)
    }

    /// Compile all rules for a class (including inherited)
    fn compile_class_rules(
        &self,
        class_name: &str,
    ) -> linkml_core::error::Result<Vec<CompiledRule>> {
        let mut inheritance_resolver = inheritance::RuleInheritanceResolver::new(&self.schema);
        let rules = inheritance_resolver.get_all_rules(class_name)?;

        let mut compiled_rules = Vec::new();
        for (rule, source_class) in rules {
            match CompiledRule::compile(rule, source_class) {
                Ok(compiled) => compiled_rules.push(compiled),
                Err(e) => {
                    // Log warning but continue with other rules
                    eprintln!("Warning: Failed to compile rule: {e}");
                }
            }
        }

        // Sort by priority (higher priority first)
        compiled_rules.sort_by(|a, b| b.priority.cmp(&a.priority));

        Ok(compiled_rules)
    }

    /// Clear the rule cache
    pub fn clear_cache(&self) {
        self.rule_cache.write().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::types::{ClassDefinition, Rule};

    #[test]
    fn test_rule_engine_creation() {
        let schema = Arc::new(SchemaDefinition::default());
        let engine = RuleEngine::new(schema);
        assert!(engine.rule_cache.read().is_empty());
    }

    #[test]
    fn test_rule_compilation() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut schema = SchemaDefinition::default();
        let mut class_def = ClassDefinition {
            name: "TestClass".to_string(),
            ..Default::default()
        };

        // Add a simple rule
        let rule = Rule {
            description: Some("Test rule".to_string()),
            priority: Some(10),
            ..Default::default()
        };
        class_def.rules.push(rule);

        schema.classes.insert("TestClass".to_string(), class_def);

        let engine = RuleEngine::new(Arc::new(schema));
        let rules = engine.get_applicable_rules("TestClass")?;
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].priority, 10);
        Ok(())
    }
}
