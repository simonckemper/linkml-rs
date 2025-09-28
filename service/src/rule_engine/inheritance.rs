//! Rule inheritance resolution
//!
//! This module handles resolving rules from parent classes and mixins,
//! managing priority adjustments, and handling rule overrides.

use linkml_core::error::{LinkMLError, Result};
use linkml_core::types::{CompositeConditions, Rule, SchemaDefinition};
use std::collections::{HashMap, HashSet, VecDeque};

/// Resolver for rule inheritance
pub struct RuleInheritanceResolver<'a> {
    schema: &'a SchemaDefinition,
    /// Cache of resolved inheritance chains
    inheritance_cache: HashMap<String, Vec<String>>,
}

impl<'a> RuleInheritanceResolver<'a> {
    /// Create a new inheritance resolver
    #[must_use]
    pub fn new(schema: &'a SchemaDefinition) -> Self {
        Self {
            schema,
            inheritance_cache: HashMap::new(),
        }
    }

    /// Get all rules for a class including inherited rules
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn get_all_rules(&mut self, class_name: &str) -> Result<Vec<(Rule, String)>> {
        // Get inheritance chain
        let inheritance_chain = self.get_inheritance_chain(class_name)?;

        // Collect rules from all classes in the chain
        let mut all_rules = Vec::new();
        let mut seen_rules = HashSet::new();

        // Process from most specific to most general
        for ancestor_name in &inheritance_chain {
            if let Some(class_def) = self.schema.classes.get(ancestor_name) {
                for rule in &class_def.rules {
                    // Create a rule identifier for deduplication
                    let rule_id = self.get_rule_id(rule);

                    // Skip if we've already seen this rule (override from more specific class)
                    if seen_rules.contains(&rule_id) {
                        continue;
                    }

                    seen_rules.insert(rule_id);

                    // Adjust priority for inherited rules
                    let adjusted_rule = if ancestor_name == class_name {
                        rule.clone()
                    } else {
                        self.adjust_inherited_rule(rule, ancestor_name, class_name)?
                    };

                    all_rules.push((adjusted_rule, ancestor_name.clone()));
                }
            }
        }

        Ok(all_rules)
    }

    /// Get the inheritance chain for a class (including mixins)
    fn get_inheritance_chain(
        &mut self,
        class_name: &str,
    ) -> linkml_core::error::Result<Vec<String>> {
        // Check cache
        if let Some(cached) = self.inheritance_cache.get(class_name) {
            return Ok(cached.clone());
        }

        // Build inheritance chain
        let mut chain = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        // Start with the class itself
        queue.push_back(class_name.to_string());

        while let Some(current_class) = queue.pop_front() {
            // Skip if already visited (handles diamond inheritance)
            if visited.contains(&current_class) {
                continue;
            }

            visited.insert(current_class.clone());
            chain.push(current_class.clone());

            // Get class definition
            if let Some(class_def) = self.schema.classes.get(&current_class) {
                // Add parent class
                if let Some(ref parent) = class_def.is_a {
                    queue.push_back(parent.clone());
                }

                // Add mixins (processed after parent for correct precedence)
                for mixin in &class_def.mixins {
                    queue.push_back(mixin.clone());
                }
            }
        }

        // Cache the result
        self.inheritance_cache
            .insert(class_name.to_string(), chain.clone());

        Ok(chain)
    }

    /// Create a unique identifier for a rule
    fn get_rule_id(&self, rule: &Rule) -> String {
        // Use title if available, otherwise description, otherwise a hash
        if let Some(ref title) = rule.title {
            title.clone()
        } else if let Some(ref desc) = rule.description {
            // Use first 50 chars of description
            desc.chars().take(50).collect()
        } else {
            // Generate a hash based on rule content
            format!("rule_{:x}", self.hash_rule(rule))
        }
    }

    /// Generate a hash for a rule
    fn hash_rule(&self, rule: &Rule) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();

        // Hash key components
        rule.description.hash(&mut hasher);
        rule.title.hash(&mut hasher);
        rule.priority.hash(&mut hasher);

        // Hash conditions (simplified - in practice would need deep hashing)
        if let Some(ref pre) = rule.preconditions {
            format!("{pre:?}").hash(&mut hasher);
        }
        if let Some(ref post) = rule.postconditions {
            format!("{post:?}").hash(&mut hasher);
        }

        hasher.finish()
    }

    /// Adjust an inherited rule
    fn adjust_inherited_rule(
        &self,
        rule: &Rule,
        source_class: &str,
        target_class: &str,
    ) -> linkml_core::error::Result<Rule> {
        let mut adjusted = rule.clone();

        // Reduce priority for inherited rules
        // Rules from direct parent get -10, from grandparent -20, etc.
        let inheritance_distance =
            self.calculate_inheritance_distance(source_class, target_class)?;
        if let Some(priority) = adjusted.priority {
            adjusted.priority = Some(priority - (10 * inheritance_distance as i32));
        } else {
            adjusted.priority = Some(-(10 * inheritance_distance as i32));
        }

        // Add inheritance info to description
        let inheritance_note = format!(" [inherited from {source_class}]");
        adjusted.description = Some(adjusted.description.unwrap_or_default() + &inheritance_note);

        Ok(adjusted)
    }

    /// Calculate inheritance distance between two classes
    fn calculate_inheritance_distance(
        &self,
        ancestor: &str,
        descendant: &str,
    ) -> linkml_core::error::Result<usize> {
        // Simple BFS to find shortest path
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();

        queue.push_back((descendant.to_string(), 0));

        while let Some((current, distance)) = queue.pop_front() {
            if current == ancestor {
                return Ok(distance);
            }

            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());

            if let Some(class_def) = self.schema.classes.get(&current) {
                // Check parent
                if let Some(ref parent) = class_def.is_a {
                    queue.push_back((parent.clone(), distance + 1));
                }

                // Check mixins
                for mixin in &class_def.mixins {
                    queue.push_back((mixin.clone(), distance + 1));
                }
            }
        }

        // If we get here, ancestor is not actually an ancestor
        Err(LinkMLError::schema_validation(format!(
            "{ancestor} is not an ancestor of {descendant}"
        )))
    }
}

/// Rule override manager
pub struct RuleOverrideManager {
    /// Map of class -> rule overrides
    overrides: HashMap<String, HashMap<String, RuleOverride>>,
}

/// Specification for overriding an inherited rule
#[derive(Debug, Clone)]
pub struct RuleOverride {
    /// Whether to completely disable the inherited rule
    pub disable: bool,
    /// New priority (if overriding)
    pub priority: Option<i32>,
    /// Additional conditions to add
    pub additional_preconditions: Option<linkml_core::types::RuleConditions>,
    /// Additional postconditions to add
    pub additional_postconditions: Option<linkml_core::types::RuleConditions>,
}

impl Default for RuleOverrideManager {
    fn default() -> Self {
        Self::new()
    }
}

impl RuleOverrideManager {
    /// Create a new override manager
    #[must_use]
    pub fn new() -> Self {
        Self {
            overrides: HashMap::new(),
        }
    }

    /// Add an override for a specific rule in a class
    pub fn add_override(
        &mut self,
        class_name: String,
        rule_id: String,
        override_spec: RuleOverride,
    ) {
        self.overrides
            .entry(class_name)
            .or_default()
            .insert(rule_id, override_spec);
    }

    /// Apply overrides to a rule
    pub fn apply_override(&self, rule: &mut Rule, rule_id: &str, class_name: &str) -> bool {
        if let Some(class_overrides) = self.overrides.get(class_name)
            && let Some(override_spec) = class_overrides.get(rule_id)
        {
            if override_spec.disable {
                rule.deactivated = Some(true);
                return true;
            }

            if let Some(priority) = override_spec.priority {
                rule.priority = Some(priority);
            }

            // Merge additional preconditions
            if let Some(ref additional_pre) = override_spec.additional_preconditions {
                if let Some(ref mut existing_pre) = rule.preconditions {
                    // Merge the composite conditions
                    if let Some(ref add_composite) = additional_pre.composite_conditions {
                        if existing_pre.composite_conditions.is_none() {
                            existing_pre.composite_conditions =
                                Some(CompositeConditions::default());
                        }
                        if let Some(ref mut existing_composite) = existing_pre.composite_conditions
                        {
                            if let Some(ref add_any) = add_composite.any_of {
                                existing_composite.any_of = Some(
                                    existing_composite
                                        .any_of
                                        .clone()
                                        .unwrap_or_default()
                                        .into_iter()
                                        .chain(add_any.clone())
                                        .collect(),
                                );
                            }
                            if let Some(ref add_all) = add_composite.all_of {
                                existing_composite.all_of = Some(
                                    existing_composite
                                        .all_of
                                        .clone()
                                        .unwrap_or_default()
                                        .into_iter()
                                        .chain(add_all.clone())
                                        .collect(),
                                );
                            }
                            if let Some(ref add_none) = add_composite.none_of {
                                existing_composite.none_of = Some(
                                    existing_composite
                                        .none_of
                                        .clone()
                                        .unwrap_or_default()
                                        .into_iter()
                                        .chain(add_none.clone())
                                        .collect(),
                                );
                            }
                            if let Some(ref add_exactly) = add_composite.exactly_one_of {
                                existing_composite.exactly_one_of = Some(
                                    existing_composite
                                        .exactly_one_of
                                        .clone()
                                        .unwrap_or_default()
                                        .into_iter()
                                        .chain(add_exactly.clone())
                                        .collect(),
                                );
                            }
                        }
                    }
                } else {
                    rule.preconditions = Some(additional_pre.clone());
                }
            }

            // Merge additional postconditions
            if let Some(ref additional_post) = override_spec.additional_postconditions {
                if let Some(ref mut existing_post) = rule.postconditions {
                    // Merge the composite conditions
                    if let Some(ref add_composite) = additional_post.composite_conditions {
                        if existing_post.composite_conditions.is_none() {
                            existing_post.composite_conditions =
                                Some(CompositeConditions::default());
                        }
                        if let Some(ref mut existing_composite) = existing_post.composite_conditions
                        {
                            if let Some(ref add_any) = add_composite.any_of {
                                existing_composite.any_of = Some(
                                    existing_composite
                                        .any_of
                                        .clone()
                                        .unwrap_or_default()
                                        .into_iter()
                                        .chain(add_any.clone())
                                        .collect(),
                                );
                            }
                            if let Some(ref add_all) = add_composite.all_of {
                                existing_composite.all_of = Some(
                                    existing_composite
                                        .all_of
                                        .clone()
                                        .unwrap_or_default()
                                        .into_iter()
                                        .chain(add_all.clone())
                                        .collect(),
                                );
                            }
                            if let Some(ref add_none) = add_composite.none_of {
                                existing_composite.none_of = Some(
                                    existing_composite
                                        .none_of
                                        .clone()
                                        .unwrap_or_default()
                                        .into_iter()
                                        .chain(add_none.clone())
                                        .collect(),
                                );
                            }
                            if let Some(ref add_exactly) = add_composite.exactly_one_of {
                                existing_composite.exactly_one_of = Some(
                                    existing_composite
                                        .exactly_one_of
                                        .clone()
                                        .unwrap_or_default()
                                        .into_iter()
                                        .chain(add_exactly.clone())
                                        .collect(),
                                );
                            }
                        }
                    }
                } else {
                    rule.postconditions = Some(additional_post.clone());
                }
            }

            return true;
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::types::ClassDefinition;

    fn create_test_schema() -> SchemaDefinition {
        let mut schema = SchemaDefinition::default();

        // Base class with a rule
        let mut base_class = ClassDefinition {
            name: "Base".to_string(),
            ..ClassDefinition::default()
        };
        base_class.rules.push(Rule {
            title: Some("base_rule".to_string()),
            description: Some("Base validation rule".to_string()),
            priority: Some(100),
            ..Rule::default()
        });

        // Derived class with override
        let mut derived_class = ClassDefinition {
            name: "Derived".to_string(),
            is_a: Some("Base".to_string()),
            ..ClassDefinition::default()
        };
        derived_class.rules.push(Rule {
            title: Some("derived_rule".to_string()),
            description: Some("Derived validation rule".to_string()),
            priority: Some(50),
            ..Rule::default()
        });

        schema.classes.insert("Base".to_string(), base_class);
        schema.classes.insert("Derived".to_string(), derived_class);

        schema
    }

    #[test]
    fn test_inheritance_chain() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let schema = create_test_schema();
        let mut resolver = RuleInheritanceResolver::new(&schema);

        let chain = resolver
            .get_inheritance_chain("Derived")
            .expect("should get inheritance chain: {}");
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0], "Derived");
        assert_eq!(chain[1], "Base");
        Ok(())
    }

    #[test]
    fn test_rule_inheritance() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let schema = create_test_schema();
        let mut resolver = RuleInheritanceResolver::new(&schema);

        let rules = resolver
            .get_all_rules("Derived")
            .expect("should get all rules: {}");
        assert_eq!(rules.len(), 2);

        // Check that derived rule comes first
        assert_eq!(rules[0].0.title, Some("derived_rule".to_string()));
        assert_eq!(rules[0].0.priority, Some(50));

        // Check that base rule is inherited with adjusted priority
        assert_eq!(rules[1].0.title, Some("base_rule".to_string()));
        assert_eq!(rules[1].0.priority, Some(90)); // 100 - 10 for inheritance distance
        Ok(())
    }
}
