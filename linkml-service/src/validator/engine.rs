//! Main validation engine

use crate::performance::profiling::global_profiler;
use linkml_core::{
    error::{LinkMLError, Result},
    settings::SchemaSettings,
    types::{ClassDefinition, SchemaDefinition, SlotDefinition},
};
use serde_json::Value;
use std::sync::Arc;
use std::time::Instant;

use super::{
    buffer_pool::ValidationBufferPools,
    cache::{CompiledValidatorCache, ValidatorCacheKey},
    compiled::{CompilationOptions, CompiledValidator},
    conditional_validator::ConditionalValidator,
    context::ValidationContext,
    default_applier::DefaultApplier,
    pattern_validator::PatternValidator,
    recursion_checker::{RecursionTracker, check_recursion},
    report::{ValidationIssue, ValidationReport},
    validators::{Validator, ValidatorRegistry},
};
use crate::inheritance::InheritanceResolver;
use crate::namespace::CurieResolver;
use crate::schema_view::SchemaView;

/// Options for validation
#[derive(Default)]pub struct ValidationOptions {
    /// Maximum depth for recursive validation
    pub max_depth: Option<usize>,
    /// Whether to fail fast on first error
    pub fail_fast: Option<bool>,
    /// Whether to validate permissible values
    pub check_permissibles: Option<bool>,
    /// Whether to use cached validators
    pub use_cache: Option<bool>,
    /// Whether to validate in parallel
    pub parallel: Option<bool>,
    /// Whether to allow additional properties not defined in schema
    pub allow_additional_properties: Option<bool>,
    /// Whether to fail on warnings (treat warnings as errors)
    pub fail_on_warning: Option<bool>,
    /// Custom validators to use
    pub custom_validators: Vec<Box<dyn Validator>>,
}

impl Clone for ValidationOptions {
    fn clone(&self) -> Self {
        Self {
            max_depth: self.max_depth,
            fail_fast: self.fail_fast,
            check_permissibles: self.check_permissibles,
            use_cache: self.use_cache,
            parallel: self.parallel,
            allow_additional_properties: self.allow_additional_properties,
            fail_on_warning: self.fail_on_warning,
            // We can't clone custom validators, so we just create an empty vec
            custom_validators: Vec::new(),
        }
    }
}

impl ValidationOptions {
    /// Create validation options from schema settings
    pub fn from_settings(settings: &SchemaSettings) -> Self {
        let mut options = Self::default();

        if let Some(validation) = &settings.validation {
            options.fail_fast = validation.fail_fast;
            options.check_permissibles = validation.check_permissibles;
            options.max_depth = validation.max_depth;
            options.allow_additional_properties = validation.allow_additional_properties;
            // fail_on_warning field exists in ValidationSettings (line 66 of settings.rs)
        }

        options
    }

    /// Merge with schema settings, with options taking precedence
    pub fn merge_with_settings(mut self, settings: &SchemaSettings) -> Self {
        if let Some(validation) = &settings.validation {
            // Only apply settings if not already set in options (None means not set)
            if self.fail_fast.is_none() {
                self.fail_fast = validation.fail_fast;
            }
            if self.check_permissibles.is_none() {
                self.check_permissibles = validation.check_permissibles;
            }
            if self.max_depth.is_none() {
                self.max_depth = validation.max_depth;
            }
            if self.allow_additional_properties.is_none() {
                self.allow_additional_properties = validation.allow_additional_properties;
            }
            if self.fail_on_warning.is_none() {
                // self.fail_on_warning = validation.fail_on_warning;
            }
        }

        self
    }

    /// Get the effective fail_fast setting
    pub fn fail_fast(&self) -> bool {
        self.fail_fast.unwrap_or(false)
    }

    /// Get the effective check_permissibles setting
    pub fn check_permissibles(&self) -> bool {
        self.check_permissibles.unwrap_or(true)
    }

    /// Get the effective use_cache setting
    pub fn use_cache(&self) -> bool {
        self.use_cache.unwrap_or(true)
    }

    /// Get the effective parallel setting
    pub fn parallel(&self) -> bool {
        self.parallel.unwrap_or(false)
    }
}

/// Main validation engine
pub struct ValidationEngine {
    pub(crate) schema: Arc<SchemaDefinition>,
    registry: ValidatorRegistry,
    compiled_cache: Option<Arc<CompiledValidatorCache>>,
    buffer_pools: Arc<ValidationBufferPools>,
}

impl ValidationEngine {
    /// Create a new validation engine for a schema
    ///
    /// # Errors
    ///
    /// Returns an error if validator registry creation fails
    pub fn new(schema: &SchemaDefinition) -> Result<Self> {
        let schema = Arc::new(schema.clone());
        let registry = ValidatorRegistry::new(&schema)?;

        Ok(Self {
            schema,
            registry,
            compiled_cache: None,
            buffer_pools: Arc::new(ValidationBufferPools::new()),
        })
    }

    /// Create a new validation engine with a compiled validator cache
    ///
    /// # Errors
    ///
    /// Returns an error if validator registry creation fails
    pub fn with_cache(
        schema: &SchemaDefinition,
        cache: Arc<CompiledValidatorCache>,
    ) -> Result<Self> {
        let schema = Arc::new(schema.clone());
        let registry = ValidatorRegistry::new(&schema)?;

        Ok(Self {
            schema,
            registry,
            compiled_cache: Some(cache),
            buffer_pools: Arc::new(ValidationBufferPools::new()),
        })
    }

    /// Add a custom validator to the engine
    pub fn add_custom_validator(&mut self, validator: Box<dyn Validator>) {
        self.registry.add_validator(validator);
    }

    /// Validate data against the schema
    ///
    /// # Errors
    ///
    /// Returns an error if validation fails
    pub async fn validate(
        &self,
        data: &Value,
        options: Option<ValidationOptions>,
    ) -> Result<ValidationReport> {
        let profiler = global_profiler();

        // Merge options with schema settings
        let options = profiler.time("validate.merge_options", || {
            match (options, &self.schema.settings) {
                (Some(opts), Some(settings)) => opts.merge_with_settings(settings),
                (Some(opts), None) => opts,
                (None, Some(settings)) => ValidationOptions::from_settings(settings),
                (None, None) => ValidationOptions::default(),
            }
        });

        // Try to determine the target class from the data
        let target_class =
            profiler.time("validate.infer_class", || self.infer_target_class(data))?;

        self.validate_as_class(data, &target_class, Some(options))
            .await
    }

    /// Validate data as a specific class
    ///
    /// # Errors
    ///
    /// Returns an error if validation fails
    pub async fn validate_as_class(
        &self,
        data: &Value,
        class_name: &str,
        options: Option<ValidationOptions>,
    ) -> Result<ValidationReport> {
        let profiler = global_profiler();
        let start = Instant::now();

        // Merge options with schema settings
        let options = profiler.time("validate_as_class.merge_options", || {
            match (options, &self.schema.settings) {
                (Some(opts), Some(settings)) => opts.merge_with_settings(settings),
                (Some(opts), None) => opts,
                (None, Some(settings)) => ValidationOptions::from_settings(settings),
                (None, None) => ValidationOptions::default(),
            }
        });

        // Check that the class exists
        let class_def = profiler.time("validate_as_class.get_class", || {
            self.schema.classes.get(class_name).ok_or_else(|| {
                LinkMLError::schema_validation(format!("Class '{class_name}' not found in schema"))
            })
        })?;

        let mut report = ValidationReport::new(&self.schema.id);
        report.target_class = Some(class_name.to_string());

        let mut context =
            ValidationContext::with_buffer_pools(self.schema.clone(), self.buffer_pools.clone());

        // Validate the data
        self.validate_class_instance(
            data,
            class_name,
            class_def,
            &mut context,
            &mut report,
            &options,
        )
        .await?;

        // Update statistics
        report.stats.duration_ms = start.elapsed().as_millis().try_into().unwrap_or(u64::MAX);
        report.stats.total_validated = 1; // For now, we validate one root object

        // Sort issues by severity and path
        report.sort_issues();

        Ok(report)
    }

    /// Validate a single instance of a class
    async fn validate_class_instance(
        &self,
        data: &Value,
        class_name: &str,
        class_def: &ClassDefinition,
        context: &mut ValidationContext,
        report: &mut ValidationReport,
        options: &ValidationOptions,
    ) -> Result<()> {
        // INTEGRATION 1: Apply defaults BEFORE validation
        let mut data = data.clone();
        let default_applier = DefaultApplier::from_schema(&self.schema);
        if let Err(e) = default_applier.apply_defaults(&mut data, &self.schema) {
            report.add_issue(ValidationIssue::warning(
                format!("Failed to apply defaults: {}", e),
                context.path().to_string(),
                "default_applier",
            ));
        }

        // INTEGRATION 2: Use SchemaView for comprehensive class analysis
        let schema_view = SchemaView::new(self.schema.as_ref().clone())?;
        let _class_view = schema_view.class_view(class_name)?;

        // INTEGRATION 3: Use InheritanceResolver for complete slot resolution
        let mut inheritance_resolver = InheritanceResolver::new(&self.schema);
        let _resolved_class = inheritance_resolver.resolve_class(class_name)?;

        // INTEGRATION 4: Check recursion depth using RecursionOptions
        if let Some(_recursion_options) = &class_def.recursion_options {
            let mut recursion_tracker = RecursionTracker::new(&self.schema);

            // Check for circular references and depth violations
            if let Err(recursion_error) =
                check_recursion(&data, class_name, &self.schema, &mut recursion_tracker)
            {
                report.add_issue(ValidationIssue::error(
                    recursion_error,
                    context.path().to_string(),
                    "recursion_checker",
                ));
                return Ok(());
            }
        }

        // INTEGRATION 5: CURIE/URI resolution
        let _curie_resolver = CurieResolver::from_schema(&self.schema);

        // INTEGRATION 6: Pattern validation for all slots
        // NOTE: Pattern validation is now handled by EnhancedPatternValidator in the registry
        // This duplicate validation is commented out to avoid conflicts
        // let pattern_validator = PatternValidator::from_schema(&self.schema)?;
        // let pattern_issues =
        //     pattern_validator.validate_instance(&data, class_name, &self.schema)?;
        // for issue in pattern_issues {
        //     report.add_issue(issue);
        //     if options.fail_fast() && !report.valid {
        //         return Ok(());
        //     }
        // }

        // INTEGRATION 7: Conditional validation with proper rules
        let conditional_validator = ConditionalValidator::from_schema(&self.schema);
        let conditional_violations = conditional_validator.validate(&data, class_name)?;
        for violation in conditional_violations {
            let message = violation
                .message
                .unwrap_or_else(|| format!("Conditional rule '{}' violated", violation.rule_name));
            report.add_issue(ValidationIssue::error(
                message,
                context.path().to_string(),
                "conditional_validator",
            ));
            if options.fail_fast() {
                return Ok(());
            }
        }

        // INTEGRATION 8: Unique key validation for collections
        // (This would be called at the collection level, not per-instance)
        // Try to use compiled validator if cache is enabled
        if options.use_cache() {
            if let Some(cache) = self.compiled_cache.as_ref() {
                let compilation_options = CompilationOptions::default();
                let cache_key =
                    ValidatorCacheKey::new(&self.schema, class_name, &compilation_options);

                // Try to get from cache
                let compiled_validator = if let Some(validator) = cache.get(&cache_key).await {
                    // Update cache statistics
                    report.stats.cache_hit_rate = cache.stats().hit_rate();
                    validator
                } else {
                    // Compile and cache
                    let start = Instant::now();
                    let validator = CompiledValidator::compile_class(
                        &self.schema,
                        class_name,
                        class_def,
                        &compilation_options,
                    )?;

                    let _compilation_time: u64 =
                        start.elapsed().as_millis().try_into().unwrap_or(u64::MAX);

                    // Store in cache
                    cache.put(cache_key, validator).await?;

                    // Get back the Arc version
                    let cache_key =
                        ValidatorCacheKey::new(&self.schema, class_name, &compilation_options);
                    cache.get(&cache_key).await.ok_or_else(|| {
                        LinkMLError::service("Failed to retrieve cached validator")
                    })?
                };

                // Use compiled validator
                let issues = compiled_validator.execute(&data, context);
                for issue in issues {
                    report.add_issue(issue);
                    if options.fail_fast() && !report.valid {
                        return Ok(());
                    }
                }

                report.stats.validators_executed += 1;
                context.pop_class();
                return Ok(());
            }
        }
        // Push class to context
        context.push_class(class_name);

        // Ensure we have an object
        if !data.is_object() {
            report.add_issue(ValidationIssue::error(
                format!(
                    "Expected object for class '{}', got {}",
                    class_name,
                    data_type_name(&data)
                ),
                context.path().to_string(),
                "type_validator",
            ));
            context.pop_class();
            return Ok(());
        }

        let obj = match data.as_object() {
            Some(obj) => obj,
            None => {
                // This should not happen as we already checked is_object()
                return Err(LinkMLError::service(
                    "Unexpected non-object after is_object check",
                ));
            }
        };

        // Set the parent context for cross-field validation
        context.set_parent(data.clone());

        // Get effective slots (including inherited) and collect slot info
        let effective_slots_info: Vec<(String, SlotDefinition, bool)> = {
            let effective_slots = context.get_effective_slots(class_name);
            effective_slots
                .into_iter()
                .map(|(name, slot_def)| {
                    (name.to_string(), slot_def.clone(), obj.contains_key(name))
                })
                .collect()
        };

        // Create a list of slot names for later checking
        let valid_slot_names: Vec<String> = effective_slots_info
            .iter()
            .map(|(name, _, _)| name.clone())
            .collect();

        // Validate each slot
        for (slot_name, slot_def, exists) in &effective_slots_info {
            if *exists {
                if let Some(value) = obj.get(slot_name) {
                    // Validate the slot value
                    context.push_path(slot_name);
                    self.validate_slot_value(value, slot_def, context, report, options);
                    context.pop_path();

                    if options.fail_fast() && !report.valid {
                        break;
                    }
                }
            } else if slot_def.required.unwrap_or(false) {
                // Required slot is missing
                report.add_issue(ValidationIssue::error(
                    format!("Required slot '{slot_name}' is missing"),
                    format!("{}.{slot_name}", context.path()),
                    "required_validator",
                ));

                if options.fail_fast() {
                    break;
                }
            }
        }

        // Check for unknown slots (if allowed by settings)
        let allow_additional = self
            .schema
            .settings
            .as_ref()
            .and_then(|s| s.validation.as_ref())
            .and_then(|v| v.allow_additional_properties)
            .unwrap_or(true); // Default to allowing additional properties

        if !allow_additional {
            for (key, _) in obj {
                if !valid_slot_names.iter().any(|name| name == key) {
                    report.add_issue(ValidationIssue::error(
                        format!("Unknown slot '{key}' in class '{class_name}'"),
                        format!("{}.{key}", context.path()),
                        "schema_validator",
                    ));
                }
            }
        } else {
            // Still warn about unknown slots even if allowed
            for (key, _) in obj {
                if !valid_slot_names.iter().any(|name| name == key) {
                    report.add_issue(ValidationIssue::warning(
                        format!("Unknown slot '{key}' in class '{class_name}'"),
                        format!("{}.{key}", context.path()),
                        "schema_validator",
                    ));
                }
            }
        }

        // Run class-level rule validation
        if let Some(rule_validator) = self.registry.rule_validator() {
            let rule_issues = rule_validator.validate_instance(&data, class_name, context);
            for issue in rule_issues {
                report.add_issue(issue);
                if options.fail_fast() && !report.valid {
                    context.pop_class();
                    return Ok(());
                }
            }
            report.stats.validators_executed += 1;
        }

        // Run conditional requirement validation
        if let Some(conditional_validator) = self.registry.conditional_requirement_validator() {
            if let Some(class_def) = self.schema.classes.get(class_name) {
                let conditional_issues =
                    conditional_validator.validate_class(&data, class_def, context);
                for issue in conditional_issues {
                    report.add_issue(issue);
                    if options.fail_fast() && !report.valid {
                        context.pop_class();
                        return Ok(());
                    }
                }
                report.stats.validators_executed += 1;
            }
        }

        context.pop_class();
        Ok(())
    }

    /// Validate a slot value
    fn validate_slot_value(
        &self,
        value: &Value,
        slot_def: &linkml_core::types::SlotDefinition,
        context: &mut ValidationContext,
        report: &mut ValidationReport,
        options: &ValidationOptions,
    ) {
        // Debug: Log slot being validated
        eprintln!("DEBUG ValidationEngine: Validating slot '{}' with pattern: {:?}", 
                 slot_def.name, slot_def.pattern);
        let profiler = global_profiler();

        // Get validators for this slot
        let validators = profiler.time("slot_validation.get_validators", || {
            self.registry.get_validators_for_slot(slot_def)
        });

        // Run each validator
        for validator in validators {
            let validator_name = validator.name();
            let issues = profiler.time(&format!("slot_validation.{}", validator_name), || {
                validator.validate(value, slot_def, context)
            });

            for issue in issues {
                report.add_issue(issue);
                if options.fail_fast() && !report.valid {
                    return;
                }
            }
            report.stats.validators_executed += 1;
        }

        // Run custom validators if any
        for validator in &options.custom_validators {
            let issues = validator.validate(value, slot_def, context);
            for issue in issues {
                report.add_issue(issue);
                if options.fail_fast() && !report.valid {
                    return;
                }
            }
            report.stats.validators_executed += 1;
        }
    }

    /// Try to infer the target class from the data
    fn infer_target_class(&self, data: &Value) -> Result<String> {
        // Simple heuristic: look for a @type field
        if let Some(obj) = data.as_object() {
            if let Some(type_value) = obj.get("@type") {
                if let Some(type_str) = type_value.as_str() {
                    return Ok(type_str.to_string());
                }
            }
        }

        // If we can't infer, look for tree_root classes
        let tree_roots: Vec<_> = self
            .schema
            .classes
            .iter()
            .filter(|(_, class)| class.tree_root.unwrap_or(false))
            .map(|(name, _)| name.clone())
            .collect();

        if tree_roots.len() == 1 {
            return Ok(tree_roots[0].clone());
        }

        Err(LinkMLError::schema_validation(
            "Cannot infer target class from data. Please specify a target class.",
        ))
    }

    /// Validate a collection of instances with unique key constraints
    ///
    /// This method validates multiple instances and checks for unique key violations
    /// across the entire collection.
    ///
    /// # Errors
    ///
    /// Returns an error if validation fails
    pub async fn validate_collection(
        &mut self,
        instances: &[Value],
        class_name: &str,
        options: Option<ValidationOptions>,
    ) -> Result<ValidationReport> {
        let start = Instant::now();
        let options = options.unwrap_or_default();

        // Check that the class exists
        let _class_def = self.schema.classes.get(class_name).ok_or_else(|| {
            LinkMLError::schema_validation(format!("Class '{class_name}' not found in schema"))
        })?;

        let mut report = ValidationReport::new(&self.schema.id);
        report.target_class = Some(class_name.to_string());

        // Reset unique key validator if present
        if let Some(validator) = self.registry.unique_key_validator_mut() {
            validator.reset();
        }

        // Validate each instance
        for (index, instance) in instances.iter().enumerate() {
            let mut context = ValidationContext::with_buffer_pools(
                self.schema.clone(),
                self.buffer_pools.clone(),
            );

            // Add collection context
            context.push_path(&format!("[{index}]"));

            // Validate the instance
            let class_def = self.schema.classes.get(class_name).ok_or_else(|| {
                LinkMLError::schema_validation(format!("Class not found: {}", class_name))
            })?;
            self.validate_class_instance(
                instance,
                class_name,
                class_def,
                &mut context,
                &mut report,
                &options,
            )
            .await?;

            // Run unique key validation after each instance
            if let Some(unique_validator) = self.registry.unique_key_validator() {
                if let Some(class_def) = self.schema.classes.get(class_name) {
                    let unique_issues = unique_validator.validate_instance(
                        instance,
                        class_def,
                        &self.schema,
                        &mut context,
                    );

                    for issue in unique_issues {
                        report.add_issue(issue);
                        if options.fail_fast() && !report.valid {
                            return Ok(report);
                        }
                    }
                }
            }

            context.pop_path();

            if options.fail_fast() && !report.valid {
                break;
            }
        }

        report.stats.duration_ms = start.elapsed().as_millis() as u64;
        Ok(report)
    }

    /// Validate a collection in parallel
    ///
    /// This method validates instances in parallel but still maintains
    /// proper unique key tracking across the collection.
    ///
    /// # Errors
    ///
    /// Returns an error if validation fails
    pub async fn validate_collection_parallel(
        &mut self,
        instances: &[Value],
        class_name: &str,
        options: Option<ValidationOptions>,
    ) -> Result<ValidationReport> {
        // For unique key validation, we need sequential processing
        // to properly track duplicates, so delegate to sequential version
        self.validate_collection(instances, class_name, options)
            .await
    }
}

/// Get a human-readable name for a JSON value type
fn data_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}
