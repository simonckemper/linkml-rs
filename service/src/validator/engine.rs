//! Main validation engine

use crate::performance::profiling::Profiler;
use crate::utils::safe_cast::u128_to_u64_saturating;
use linkml_core::{
    error::{LinkMLError, Result},
    settings::SchemaSettings,
    types::{ClassDefinition, SchemaDefinition, SlotDefinition},
};
use serde_json::Value;
use std::sync::Arc;
use timestamp_core::SyncTimestampService;

use super::{
    buffer_pool::ValidationBufferPools,
    cache::{CompiledValidatorCache, ValidatorCacheKey},
    compiled::{CompilationOptions, CompiledValidator},
    conditional_validator::ConditionalValidator,
    context::ValidationContext,
    default_applier::DefaultApplier,
    recursion_checker::{RecursionTracker, check_recursion},
    report::{ValidationIssue, ValidationReport},
    validators::{Validator, ValidatorRegistry},
};
use crate::inheritance::InheritanceResolver;
use crate::namespace::CurieResolver;
use crate::schema_view::SchemaView;

/// Options for validation
#[derive(Default)]
pub struct ValidationOptions {
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
    #[must_use]
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
    #[must_use]
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

    /// Get the effective `fail_fast` setting
    #[must_use]
    pub fn fail_fast(&self) -> bool {
        self.fail_fast.unwrap_or(false)
    }

    /// Get the effective `check_permissibles` setting
    #[must_use]
    pub fn check_permissibles(&self) -> bool {
        self.check_permissibles.unwrap_or(true)
    }

    /// Get the effective `use_cache` setting
    #[must_use]
    pub fn use_cache(&self) -> bool {
        self.use_cache.unwrap_or(true)
    }

    /// Get the effective parallel setting
    #[must_use]
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
    timestamp_service: Arc<dyn SyncTimestampService<Error = timestamp_core::TimestampError>>,
    profiler: Arc<Profiler>,
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
        let timestamp_service = timestamp_service::wiring::wire_timestamp();
        let profiler = Arc::new(Profiler::new(
            timestamp_service::wiring::wire_timestamp().into_inner(),
        ));

        Ok(Self {
            schema,
            registry,
            compiled_cache: None,
            buffer_pools: Arc::new(ValidationBufferPools::new()),
            timestamp_service: timestamp_service.into_inner(),
            profiler,
        })
    }

    /// Create a new validation engine with injected timestamp service (factory pattern compliant)
    ///
    /// # Errors
    ///
    /// Returns an error if validator registry creation fails
    pub fn with_timestamp_service<T>(
        schema: &SchemaDefinition,
        timestamp_service: Arc<T>,
    ) -> Result<Self>
    where
        T: SyncTimestampService<Error = timestamp_core::TimestampError> + Send + Sync + 'static,
    {
        let schema = Arc::new(schema.clone());
        let registry = ValidatorRegistry::new(&schema)?;

        let profiler = Arc::new(Profiler::new(
            timestamp_service::wiring::wire_timestamp().into_inner(),
        ));

        Ok(Self {
            schema,
            registry,
            compiled_cache: None,
            buffer_pools: Arc::new(ValidationBufferPools::new()),
            timestamp_service,
            profiler,
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
        let timestamp_service = timestamp_service::wiring::wire_timestamp();

        Ok(Self {
            schema,
            registry,
            compiled_cache: Some(cache),
            buffer_pools: Arc::new(ValidationBufferPools::new()),
            timestamp_service: timestamp_service.clone(),
            profiler: Arc::new(Profiler::new(Arc::new(
                timestamp_service::wiring::wire_timestamp(),
            ))),
        })
    }

    /// Create a new validation engine with cache and injected timestamp service (factory pattern compliant)
    ///
    /// # Errors
    ///
    /// Returns an error if validator registry creation fails
    pub fn with_cache_and_timestamp(
        schema: &SchemaDefinition,
        cache: Arc<CompiledValidatorCache>,
        timestamp_service: Arc<dyn SyncTimestampService<Error = timestamp_core::TimestampError>>,
    ) -> Result<Self> {
        let schema = Arc::new(schema.clone());
        let registry = ValidatorRegistry::new(&schema)?;

        Ok(Self {
            schema,
            registry,
            compiled_cache: Some(cache),
            buffer_pools: Arc::new(ValidationBufferPools::new()),
            timestamp_service,
            profiler: Arc::new(Profiler::new(Arc::new(
                timestamp_service::wiring::wire_timestamp(),
            ))),
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
        let profiler = &self.profiler;

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
        let profiler = &self.profiler;
        let start = self
            .timestamp_service
            .system_time()
            .map_err(|e| LinkMLError::service(format!("Failed to get system time: {e}")))?;

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
        let end = self
            .timestamp_service
            .system_time()
            .map_err(|e| LinkMLError::service(format!("Failed to get system time: {e}")))?;
        let duration = end
            .duration_since(start)
            .map_err(|e| LinkMLError::service(format!("Time calculation error: {e}")))?;
        report.stats.duration_ms = duration.as_millis().try_into().unwrap_or(u64::MAX);
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
        let data = self.apply_defaults_and_prepare(data, context, report);

        self.setup_schema_analysis(class_name)?;
        self.check_recursion_constraints(&data, class_name, class_def, context, report);

        if self.handle_recursion_guard(&data, class_name, class_def, context, report) {
            return Ok(());
        }

        let _curie_resolver = CurieResolver::from_schema(&self.schema);

        if self.run_conditional_rules(&data, class_name, context, report, options)? {
            return Ok(());
        }

        if self
            .try_validate_with_compiled_validator(
                &data, class_name, class_def, context, report, options,
            )
            .await?
        {
            return Ok(());
        }

        context.push_class(class_name);

        let Some(obj) = Self::ensure_object_for_class(&data, class_name, context, report)? else {
            context.pop_class();
            return Ok(());
        };

        let valid_slot_names =
            self.validate_declared_slots(&data, obj, class_name, context, report, options);

        self.audit_unknown_slots(obj, class_name, context, &valid_slot_names, report);

        if self.run_class_level_validators(&data, class_name, class_def, context, report, options) {
            context.pop_class();
            return Ok(());
        }

        context.pop_class();
        Ok(())
    }

    fn handle_recursion_guard(
        &self,
        data: &Value,
        class_name: &str,
        class_def: &ClassDefinition,
        context: &ValidationContext,
        report: &mut ValidationReport,
    ) -> bool {
        if let Some(_recursion_options) = &class_def.recursion_options {
            let mut recursion_tracker = RecursionTracker::new(&self.schema);

            if let Err(recursion_error) =
                check_recursion(data, class_name, &self.schema, &mut recursion_tracker)
            {
                report.add_issue(ValidationIssue::error(
                    recursion_error,
                    context.path(),
                    "recursion_checker",
                ));
                return true;
            }
        }

        false
    }

    fn run_conditional_rules(
        &self,
        data: &Value,
        class_name: &str,
        context: &ValidationContext,
        report: &mut ValidationReport,
        options: &ValidationOptions,
    ) -> Result<bool> {
        let conditional_validator = ConditionalValidator::from_schema(&self.schema);
        let conditional_violations = conditional_validator.validate(data, class_name)?;

        for violation in conditional_violations {
            let message = violation
                .message
                .unwrap_or_else(|| format!("Conditional rule '{}' violated", violation.rule_name));
            report.add_issue(ValidationIssue::error(
                message,
                context.path(),
                "conditional_validator",
            ));
            if options.fail_fast() {
                return Ok(true);
            }
        }

        Ok(false)
    }

    async fn try_validate_with_compiled_validator(
        &self,
        data: &Value,
        class_name: &str,
        class_def: &ClassDefinition,
        context: &mut ValidationContext,
        report: &mut ValidationReport,
        options: &ValidationOptions,
    ) -> Result<bool> {
        if !options.use_cache() {
            return Ok(false);
        }

        let Some(cache) = self.compiled_cache.as_ref() else {
            return Ok(false);
        };

        let compilation_options = CompilationOptions::default();
        let cache_key = ValidatorCacheKey::new(&self.schema, class_name, &compilation_options);

        let compiled_validator = if let Some(validator) = cache.get(&cache_key).await {
            report.stats.cache_hit_rate = cache.stats().hit_rate();
            validator
        } else {
            let compile_start = self
                .timestamp_service
                .system_time()
                .map_err(|e| LinkMLError::service(format!("Failed to get system time: {e}")))?;

            let validator = CompiledValidator::compile_class(
                &self.schema,
                class_name,
                class_def,
                compilation_options,
            )?;

            let compile_end = self
                .timestamp_service
                .system_time()
                .map_err(|e| LinkMLError::service(format!("Failed to get system time: {e}")))?;
            let _compilation_time: u64 = compile_end
                .duration_since(compile_start)
                .map_err(|e| LinkMLError::service(format!("Time calculation error: {e}")))?
                .as_millis()
                .try_into()
                .unwrap_or(u64::MAX);

            cache.put(&cache_key, validator)?;

            let cache_key = ValidatorCacheKey::new(&self.schema, class_name, &compilation_options);
            cache
                .get(&cache_key)
                .await
                .ok_or_else(|| LinkMLError::service("Failed to retrieve cached validator"))?
        };

        let issues = compiled_validator.execute(data, context);
        for issue in issues {
            report.add_issue(issue);
            if options.fail_fast() && !report.valid {
                return Ok(true);
            }
        }

        report.stats.validators_executed += 1;
        context.pop_class();
        Ok(true)
    }

    fn ensure_object_for_class<'a>(
        data: &'a Value,
        class_name: &str,
        context: &ValidationContext,
        report: &mut ValidationReport,
    ) -> Result<Option<&'a serde_json::Map<String, Value>>> {
        if !data.is_object() {
            report.add_issue(ValidationIssue::error(
                format!(
                    "Expected object for class '{}', got {}",
                    class_name,
                    data_type_name(data)
                ),
                context.path(),
                "type_validator",
            ));
            return Ok(None);
        }

        let obj = data
            .as_object()
            .ok_or_else(|| LinkMLError::service("Unexpected non-object after is_object check"))?;

        Ok(Some(obj))
    }

    fn validate_declared_slots(
        &self,
        data: &Value,
        obj: &serde_json::Map<String, Value>,
        class_name: &str,
        context: &mut ValidationContext,
        report: &mut ValidationReport,
        options: &ValidationOptions,
    ) -> Vec<String> {
        context.set_parent(data.clone());
        let effective_slots: Vec<(String, SlotDefinition)> = context
            .get_effective_slots(class_name)
            .into_iter()
            .map(|(name, slot_def)| (name.to_string(), slot_def.clone()))
            .collect();
        let valid_slot_names: Vec<String> = effective_slots
            .iter()
            .map(|(name, _)| name.clone())
            .collect();

        for (name, slot_def) in &effective_slots {
            if let Some(value) = obj.get(name.as_str()) {
                context.push_path(name.clone());
                self.validate_slot_value(value, slot_def, context, report, options);
                context.pop_path();

                if options.fail_fast() && !report.valid {
                    break;
                }
            } else if slot_def.required.unwrap_or(false) {
                report.add_issue(ValidationIssue::error(
                    format!("Required slot '{name}' is missing"),
                    format!("{}.{name}", context.path()),
                    "required_validator",
                ));

                if options.fail_fast() {
                    break;
                }
            }
        }

        valid_slot_names
    }

    fn audit_unknown_slots(
        &self,
        obj: &serde_json::Map<String, Value>,
        class_name: &str,
        context: &ValidationContext,
        valid_slot_names: &[String],
        report: &mut ValidationReport,
    ) {
        let allow_additional = self
            .schema
            .settings
            .as_ref()
            .and_then(|s| s.validation.as_ref())
            .and_then(|v| v.allow_additional_properties)
            .unwrap_or(true);

        for key in obj.keys() {
            if valid_slot_names.iter().any(|name| name == key) {
                continue;
            }

            let issue = if allow_additional {
                ValidationIssue::warning(
                    format!("Unknown slot '{key}' in class '{class_name}'"),
                    format!("{}.{key}", context.path()),
                    "schema_validator",
                )
            } else {
                ValidationIssue::error(
                    format!("Unknown slot '{key}' in class '{class_name}'"),
                    format!("{}.{key}", context.path()),
                    "schema_validator",
                )
            };

            report.add_issue(issue);
        }
    }

    fn run_class_level_validators(
        &self,
        data: &Value,
        class_name: &str,
        class_def: &ClassDefinition,
        context: &mut ValidationContext,
        report: &mut ValidationReport,
        options: &ValidationOptions,
    ) -> bool {
        if let Some(rule_validator) = self.registry.rule_validator() {
            let rule_issues = rule_validator.validate_instance(data, class_name, context);
            for issue in rule_issues {
                report.add_issue(issue);
                if options.fail_fast() && !report.valid {
                    return true;
                }
            }
            report.stats.validators_executed += 1;
        }

        if let Some(conditional_validator) = self.registry.conditional_requirement_validator() {
            let conditional_issues = conditional_validator.validate_class(data, class_def, context);
            for issue in conditional_issues {
                report.add_issue(issue);
                if options.fail_fast() && !report.valid {
                    return true;
                }
            }
            report.stats.validators_executed += 1;
        }

        false
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
        eprintln!(
            "DEBUG ValidationEngine: Validating slot '{}' with pattern: {:?}",
            slot_def.name, slot_def.pattern
        );
        let profiler = &self.profiler;

        // Get validators for this slot
        let validators = profiler.time("slot_validation.get_validators", || {
            self.registry.get_validators_for_slot(slot_def)
        });

        // Run each validator
        for validator in validators {
            let validator_name = validator.name();
            let issues = profiler.time(&format!("slot_validation.{validator_name}"), || {
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
        if let Some(obj) = data.as_object()
            && let Some(type_value) = obj.get("@type")
            && let Some(type_str) = type_value.as_str()
        {
            return Ok(type_str.to_string());
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
        let start = self
            .timestamp_service
            .system_time()
            .map_err(|e| LinkMLError::service(format!("Failed to get system time: {e}")))?;
        let options = options.unwrap_or_default();

        // Check that the class exists
        let _class_def = self.schema.classes.get(class_name).ok_or_else(|| {
            LinkMLError::schema_validation(format!("Class '{class_name}' not found in schema"))
        })?;

        let mut report = ValidationReport::new(&self.schema.id);
        report.target_class = Some(class_name.to_string());

        // Reset unique key validator if present
        if let Some(validator) = self.registry.unique_key_validator_mut() {
            let _ = validator.reset();
        }

        // Validate each instance
        for (index, instance) in instances.iter().enumerate() {
            let mut context = ValidationContext::with_buffer_pools(
                self.schema.clone(),
                self.buffer_pools.clone(),
            );

            // Add collection context
            context.push_path(format!("[{index}]"));

            // Validate the instance
            let class_def = self.schema.classes.get(class_name).ok_or_else(|| {
                LinkMLError::schema_validation(format!("Class not found: {class_name}"))
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
            if let Some(unique_validator) = self.registry.unique_key_validator()
                && let Some(class_def) = self.schema.classes.get(class_name)
            {
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

            context.pop_path();

            if options.fail_fast() && !report.valid {
                break;
            }
        }

        let end = self
            .timestamp_service
            .system_time()
            .map_err(|e| LinkMLError::service(format!("Failed to get system time: {e}")))?;
        let duration = end
            .duration_since(start)
            .map_err(|e| LinkMLError::service(format!("Time calculation error: {e}")))?;
        report.stats.duration_ms = u128_to_u64_saturating(duration.as_millis());
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

    /// Apply defaults and prepare data for validation
    fn apply_defaults_and_prepare(
        &self,
        data: &Value,
        context: &ValidationContext,
        report: &mut ValidationReport,
    ) -> Value {
        let mut data = data.clone();
        let default_applier = DefaultApplier::from_schema(&self.schema);
        if let Err(e) = default_applier.apply_defaults(&mut data, &self.schema) {
            report.add_issue(ValidationIssue::warning(
                format!("Failed to apply defaults: {e}"),
                context.path(),
                "default_applier",
            ));
        }
        data
    }

    /// Setup schema analysis components
    fn setup_schema_analysis(&self, class_name: &str) -> Result<()> {
        // Use SchemaView for comprehensive class analysis
        let schema_view = SchemaView::new(self.schema.as_ref().clone())?;
        let _class_view = schema_view.class_view(class_name)?;

        // Use InheritanceResolver for complete slot resolution
        let mut inheritance_resolver = InheritanceResolver::new(&self.schema);
        let _resolved_class = inheritance_resolver.resolve_class(class_name)?;

        Ok(())
    }

    /// Check recursion constraints
    fn check_recursion_constraints(
        &self,
        data: &Value,
        class_name: &str,
        class_def: &ClassDefinition,
        context: &ValidationContext,
        report: &mut ValidationReport,
    ) {
        if let Some(_recursion_options) = &class_def.recursion_options {
            let mut recursion_tracker = RecursionTracker::new(&self.schema);

            // Check for circular references and depth violations
            if let Err(recursion_error) =
                check_recursion(data, class_name, &self.schema, &mut recursion_tracker)
            {
                report.add_issue(ValidationIssue::error(
                    recursion_error,
                    context.path(),
                    "recursion_checker",
                ));
            }
        }
    }
}

/// Get a human-readable name for a `JSON` value type
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
