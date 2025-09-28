//! Rule execution strategies and orchestration
//!
//! This module handles the execution of rules based on different strategies
//! (sequential, parallel, fail-fast, etc.) and manages the overall rule
//! evaluation process.

use std::sync::Arc;

use crate::expression::ExpressionEngine;
use crate::validator::report::ValidationIssue;

use super::evaluator::RuleEvaluator;
use super::matcher::RuleMatcher;
use super::types::{CompiledRule, RuleExecutionContext, RuleExecutionStrategy};

/// Executor for rule-based validation
pub struct RuleExecutor {
    matcher: RuleMatcher,
    evaluator: RuleEvaluator,
}

impl RuleExecutor {
    /// Create a new rule executor
    #[must_use]
    pub fn new(expression_engine: Arc<ExpressionEngine>) -> Self {
        let matcher = RuleMatcher::new((*expression_engine).clone());
        let evaluator = RuleEvaluator::new((*expression_engine).clone());

        Self { matcher, evaluator }
    }

    /// Execute a set of rules with the specified strategy
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn execute_rules(
        &self,
        rules: &[CompiledRule],
        context: &mut RuleExecutionContext,
        strategy: RuleExecutionStrategy,
    ) -> linkml_core::error::Result<Vec<ValidationIssue>> {
        match strategy {
            RuleExecutionStrategy::Sequential => self.execute_sequential(rules, context),
            RuleExecutionStrategy::Parallel => self.execute_parallel(rules, context),
            RuleExecutionStrategy::FailFast => self.execute_fail_fast(rules, context),
            RuleExecutionStrategy::CollectAll => self.execute_collect_all(rules, context),
        }
    }

    /// Execute rules sequentially in priority order
    fn execute_sequential(
        &self,
        rules: &[CompiledRule],
        context: &mut RuleExecutionContext,
    ) -> linkml_core::error::Result<Vec<ValidationIssue>> {
        let mut all_issues = Vec::new();

        for rule in rules {
            if rule.deactivated {
                continue;
            }

            let issues = self.execute_single_rule(rule, context)?;
            all_issues.extend(issues);
        }

        Ok(all_issues)
    }

    /// Execute rules in parallel (future enhancement)
    fn execute_parallel(
        &self,
        rules: &[CompiledRule],
        context: &mut RuleExecutionContext,
    ) -> linkml_core::error::Result<Vec<ValidationIssue>> {
        // Note: Parallel execution is not possible with RuleExecutionContext due to mutable references
        // Falling back to sequential execution for now
        // Proper parallel execution would require:
        // 1. Context cloning for each thread to avoid mutable reference conflicts
        // 2. Result merging from parallel workers
        // 3. Synchronization of shared validation state
        // This is a complex optimization that should be implemented when performance
        // profiling indicates it's needed. Sequential execution is safer for now."
        self.execute_sequential(rules, context)
    }

    /// Execute rules but stop on first failure
    fn execute_fail_fast(
        &self,
        rules: &[CompiledRule],
        context: &mut RuleExecutionContext,
    ) -> linkml_core::error::Result<Vec<ValidationIssue>> {
        for rule in rules {
            if rule.deactivated {
                continue;
            }

            let issues = self.execute_single_rule(rule, context)?;
            if !issues.is_empty() {
                return Ok(issues);
            }
        }

        Ok(Vec::new())
    }

    /// Execute all rules and collect all issues
    fn execute_collect_all(
        &self,
        rules: &[CompiledRule],
        context: &mut RuleExecutionContext,
    ) -> linkml_core::error::Result<Vec<ValidationIssue>> {
        // This is the same as sequential for now
        self.execute_sequential(rules, context)
    }

    /// Execute a single rule
    fn execute_single_rule(
        &self,
        rule: &CompiledRule,
        context: &mut RuleExecutionContext,
    ) -> linkml_core::error::Result<Vec<ValidationIssue>> {
        let mut issues = Vec::new();

        // Set current rule for recursion detection
        let rule_id = rule
            .original
            .title
            .clone()
            .or_else(|| rule.original.description.clone())
            .unwrap_or_else(|| format!("Rule from {}", rule.source_class));

        context.current_rule = Some(rule_id.clone());

        // Check preconditions
        let preconditions_match = if let Some(ref preconditions) = rule.precondition_ast {
            self.matcher.matches(preconditions, context)?
        } else {
            // No preconditions means rule always applies
            true
        };

        if preconditions_match {
            // Mark rule as matched
            context.mark_matched(rule_id.clone());

            // Evaluate postconditions
            if let Some(ref postconditions) = rule.postcondition_ast {
                let rule_desc = rule
                    .original
                    .description
                    .as_deref()
                    .or(rule.original.title.as_deref());

                issues.extend(self.evaluator.evaluate_postconditions(
                    postconditions,
                    context,
                    rule_desc,
                )?);
            }
        } else {
            // Preconditions didn't match, check else conditions
            if let Some(ref else_conditions) = rule.else_condition_ast {
                let rule_desc = rule
                    .original
                    .description
                    .as_deref()
                    .or(rule.original.title.as_deref())
                    .map(|d| format!("{d} (else)"));

                issues.extend(self.evaluator.evaluate_postconditions(
                    else_conditions,
                    context,
                    rule_desc.as_deref(),
                )?);
            }
        }

        // Clear current rule
        context.current_rule = None;

        Ok(issues)
    }
}

/// Rule execution statistics
#[derive(Debug, Default)]
pub struct RuleExecutionStats {
    /// Total rules evaluated
    pub total_rules: usize,
    /// Rules that matched (preconditions satisfied)
    pub matched_rules: usize,
    /// Rules that were skipped (deactivated)
    pub skipped_rules: usize,
    /// Total issues generated
    pub total_issues: usize,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
}

impl RuleExecutor {
    /// Execute rules with statistics collection
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn execute_with_stats(
        &self,
        rules: &[CompiledRule],
        context: &mut RuleExecutionContext,
        strategy: RuleExecutionStrategy,
    ) -> linkml_core::error::Result<(Vec<ValidationIssue>, RuleExecutionStats)> {
        let start = std::time::Instant::now();
        let mut stats = RuleExecutionStats {
            total_rules: rules.len(),
            ..Default::default()
        };

        // Count deactivated rules
        stats.skipped_rules = rules.iter().filter(|r| r.deactivated).count();

        // Execute rules
        let issues = self.execute_rules(rules, context, strategy)?;

        // Update stats
        stats.matched_rules = context.matched_rules.len();
        stats.total_issues = issues.len();
        stats.execution_time_ms = start.elapsed().as_millis() as u64;

        Ok((issues, stats))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validator::context::ValidationContext;
    use indexmap::IndexMap;
    use linkml_core::types::{Rule, RuleConditions, SlotCondition};
    use serde_json::json;

    fn create_test_rule() -> std::result::Result<CompiledRule, Box<dyn std::error::Error>> {
        let mut slot_conditions = IndexMap::new();
        slot_conditions.insert(
            "age".to_string(),
            SlotCondition {
                minimum_value: Some(json!(18)),
                ..Default::default()
            },
        );

        let rule = Rule {
            description: Some("Adults must have ID".to_string()),
            priority: Some(10),
            preconditions: Some(RuleConditions {
                slot_conditions: Some(slot_conditions.clone()),
                ..Default::default()
            }),
            postconditions: Some(RuleConditions {
                slot_conditions: Some({
                    let mut conditions = IndexMap::new();
                    conditions.insert(
                        "id_number".to_string(),
                        SlotCondition {
                            required: Some(true),
                            ..Default::default()
                        },
                    );
                    conditions
                }),
                ..Default::default()
            }),
            ..Default::default()
        };

        Ok(CompiledRule::compile(rule, "Person".to_string())?)
    }

    #[test]
    fn test_rule_execution() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let executor = RuleExecutor::new(Arc::new(ExpressionEngine::new()));
        let rule = create_test_rule()?;

        // Test with adult without ID
        let mut validation_ctx = ValidationContext::new(Default::default());
        let mut context = RuleExecutionContext::new(
            json!({
                "age": 25,
                "name": "John"
            }),
            "Person".to_string(),
            &mut validation_ctx,
        );

        let issues = executor
            .execute_single_rule(&rule, &mut context)
            .expect("should execute rule: {}");
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("required"));

        // Test with adult with ID
        let mut validation_ctx2 = ValidationContext::new(Default::default());
        let mut context2 = RuleExecutionContext::new(
            json!({
                "age": 25,
                "name": "John",
                "id_number": "123456"
            }),
            "Person".to_string(),
            &mut validation_ctx2,
        );

        let issues2 = executor
            .execute_single_rule(&rule, &mut context2)
            .expect("should execute rule for adult with ID: {}");
        assert!(issues2.is_empty());

        // Test with minor (rule shouldn't apply)
        let mut validation_ctx3 = ValidationContext::new(Default::default());
        let mut context3 = RuleExecutionContext::new(
            json!({
                "age": 16,
                "name": "Jane"
            }),
            "Person".to_string(),
            &mut validation_ctx3,
        );

        let issues3 = executor
            .execute_single_rule(&rule, &mut context3)
            .expect("should execute rule for minor: {}");
        assert!(issues3.is_empty());
        Ok(())
    }

    #[test]
    fn test_execution_strategies() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let executor = RuleExecutor::new(Arc::new(ExpressionEngine::new()));
        let rules = vec![create_test_rule()?];

        let mut validation_ctx = ValidationContext::new(Default::default());
        let mut context = RuleExecutionContext::new(
            json!({
                "age": 25,
                "name": "John"
            }),
            "Person".to_string(),
            &mut validation_ctx,
        );

        // Test different strategies
        let sequential_issues = executor
            .execute_rules(&rules, &mut context, RuleExecutionStrategy::Sequential)
            .expect("should execute rules sequentially: {}");
        assert_eq!(sequential_issues.len(), 1);

        let mut validation_ctx2 = ValidationContext::new(Default::default());
        let mut context2 = RuleExecutionContext::new(
            json!({
                "age": 25,
                "name": "John"
            }),
            "Person".to_string(),
            &mut validation_ctx2,
        );
        let fail_fast_issues = executor
            .execute_rules(&rules, &mut context2, RuleExecutionStrategy::FailFast)
            .expect("should execute rules with fail-fast: {}");
        assert_eq!(fail_fast_issues.len(), 1);
        Ok(())
    }
}
