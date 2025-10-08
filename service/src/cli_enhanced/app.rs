//! `LinkML` enhanced CLI application.

use super::types::{
    AuthType, ConflictResolution, DiffFormat, DumpFormat, LinkMLCli, LinkMLCommand, LintFormat,
    LoadFormat, MergeStrategy, OutputFormat, SchemaFormat,
};
use crate::cli_enhanced::commands::serve::ServeCommand;
use crate::generator::{Generator, GeneratorOptions, GeneratorRegistry, IndentStyle};
use crate::schema::{
    DiffOptions, LintOptions, MergeOptions, SchemaDiff, SchemaLinter, SchemaMerge, Severity,
};
use crate::utils::timestamp::SyncTimestampUtils;
use crate::validator::engine::{ValidationEngine, ValidationOptions};
use crate::validator::report::ValidationReport;
use clap::Parser;
use linkml_core::error::{LinkMLError, Result};
use linkml_core::types::SchemaDefinition;
use serde_json::Value;
use std::collections::HashMap;
use std::fmt::Write as FmtWrite;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tracing::{error, info, warn};

/// Main `LinkML` CLI application
pub struct LinkMLApp {
    cli: LinkMLCli,
    timestamp_utils: Arc<SyncTimestampUtils>,
}

impl LinkMLApp {
    /// Create a new `LinkML` application from command line arguments with timestamp service
    #[must_use]
    pub fn from_args_with_timestamp(
        timestamp_service: Arc<
            dyn timestamp_core::TimestampService<Error = timestamp_core::TimestampError>,
        >,
    ) -> Self {
        let cli = LinkMLCli::parse();
        let timestamp_utils = Arc::new(SyncTimestampUtils::new(timestamp_service));
        Self {
            cli,
            timestamp_utils,
        }
    }

    /// Create a new `LinkML` application with custom CLI configuration and timestamp service
    #[must_use]
    pub fn new(
        cli: LinkMLCli,
        timestamp_service: Arc<
            dyn timestamp_core::TimestampService<Error = timestamp_core::TimestampError>,
        >,
    ) -> Self {
        let timestamp_utils = Arc::new(SyncTimestampUtils::new(timestamp_service));
        Self {
            cli,
            timestamp_utils,
        }
    }

    /// Create a new `LinkML` application with custom CLI configuration and timestamp service
    #[must_use]
    pub fn with_services(
        cli: LinkMLCli,
        timestamp_service: Arc<
            dyn timestamp_core::TimestampService<Error = timestamp_core::TimestampError>,
        >,
    ) -> Self {
        let timestamp_utils = Arc::new(SyncTimestampUtils::new(timestamp_service));
        Self {
            cli,
            timestamp_utils,
        }
    }

    /// Run the `LinkML` application
    ///
    /// # Errors
    ///
    /// Returns error if command execution fails or if required services cannot be initialized.
    pub async fn run(self) -> Result<()> {
        self.init_logging();
        info!("Starting LinkML CLI application");

        match self.execute_command().await {
            Ok(()) => {
                info!("Command completed successfully");
                Ok(())
            }
            Err(err) => {
                error!("Command failed: {}", err);
                if !self.cli.quiet {
                    eprintln!("Error: {err}");
                }
                Err(err)
            }
        }
    }

    /// Configure tracing subscriber based on CLI flags
    fn init_logging(&self) {
        if self.cli.quiet {
            tracing_subscriber::fmt()
                .with_max_level(tracing::Level::ERROR)
                .with_target(false)
                .init();
        } else if self.cli.verbose {
            tracing_subscriber::fmt()
                .with_max_level(tracing::Level::DEBUG)
                .with_target(false)
                .init();
        } else {
            tracing_subscriber::fmt()
                .with_max_level(tracing::Level::INFO)
                .with_target(false)
                .init();
        }
    }

    async fn execute_command(&self) -> Result<()> {
        match &self.cli.command {
            LinkMLCommand::Validate {
                schema,
                data,
                class_name,
                strict,
                max_errors,
                stats,
                parallel,
            } => {
                self.validate_command(
                    schema,
                    data,
                    class_name.as_deref(),
                    *strict,
                    *max_errors,
                    *stats,
                    *parallel,
                )
                .await
            }
            LinkMLCommand::Generate {
                schema,
                generator,
                output,
                options,
                ..
            } => {
                self.generate_command(schema, generator, output, options)
                    .await
            }
            LinkMLCommand::Convert {
                input,
                output,
                from,
                to,
                pretty,
                validate,
            } => {
                self.convert_command(input, output, *from, *to, *pretty, *validate)
                    .await
            }
            LinkMLCommand::Lint {
                schema,
                rules,
                config,
                fix,
                strict,
                format,
            } => {
                self.lint_command(schema, rules, config.as_ref(), *fix, *strict, *format)
                    .await
            }
            LinkMLCommand::Diff {
                schema1,
                schema2,
                format,
                include_docs,
                breaking_only,
                context,
                output,
            } => {
                self.diff_command(
                    schema1,
                    schema2,
                    *format,
                    *include_docs,
                    *breaking_only,
                    *context,
                    output.as_ref(),
                )
                .await
            }
            LinkMLCommand::Merge {
                schemas,
                output,
                strategy,
                conflict,
                base,
                validate,
            } => {
                self.merge_command(
                    schemas,
                    output,
                    *strategy,
                    *conflict,
                    base.as_ref(),
                    *validate,
                )
                .await
            }
            LinkMLCommand::Dump {
                schema,
                input,
                output,
                format,
                options,
                pretty,
            } => {
                self.dump_command(schema, input, output, *format, options, *pretty)
                    .await
            }
            LinkMLCommand::Load {
                schema,
                input,
                format,
                output,
                options,
                validate,
                class_name,
            } => {
                self.load_command(
                    schema,
                    input,
                    *format,
                    output,
                    options,
                    *validate,
                    class_name.as_deref(),
                )
                .await
            }
            LinkMLCommand::Serve {
                schema,
                port,
                host,
                cors,
                auth,
                ..
            } => {
                if *cors {
                    warn!(
                        "CORS configuration is delegated to the REST API service; local flag has no effect"
                    );
                }
                if matches!(
                    auth,
                    Some(AuthType::ApiKey | AuthType::Bearer | AuthType::Basic)
                ) {
                    warn!(
                        "Authentication is managed by API Gateway; local serve command runs without auth"
                    );
                }
                self.serve_command(schema, *port, host).await
            }
            LinkMLCommand::Shell { .. } => Err(LinkMLError::not_implemented(
                "Interactive shell is migrating to the Task Management framework",
            )),
            LinkMLCommand::Sheets2Schema {
                input,
                output,
                schema_id,
                schema_name,
                schema_format,
                progress,
            } => {
                self.sheets2schema_command(
                    input,
                    output.as_ref(),
                    schema_id.as_ref(),
                    schema_name.as_ref(),
                    *schema_format,
                    *progress,
                )
                .await
            }
            LinkMLCommand::Schema2Sheets {
                schema,
                output,
                validation,
                examples,
                freeze_headers,
                filters,
                progress,
            } => {
                self.schema2sheets_command(
                    schema,
                    output,
                    *validation,
                    *examples,
                    *freeze_headers,
                    *filters,
                    *progress,
                )
                .await
            }
        }
    }

    async fn validate_command(
        &self,
        schema_path: &Path,
        data_paths: &[PathBuf],
        class_name: Option<&str>,
        strict: bool,
        max_errors: usize,
        show_stats: bool,
        parallel: bool,
    ) -> Result<()> {
        let schema = self.load_schema(schema_path).await?;
        let engine = ValidationEngine::new(&schema)
            .map_err(|err| LinkMLError::service(format!("Failed to build validator: {err}")))?;

        let options = ValidationOptions {
            fail_fast: if strict { Some(true) } else { None },
            parallel: Some(parallel),
            allow_additional_properties: None,
            max_depth: None,
            check_permissibles: None,
            use_cache: Some(true),
            fail_on_warning: if strict { Some(true) } else { None },
            custom_validators: Vec::new(),
        };

        let mut any_failures = false;
        for data_path in data_paths {
            let value = self.load_data_value(data_path).await?;
            let mut report = if let Some(target) = class_name {
                engine
                    .validate_as_class(&value, target, Some(options.clone()))
                    .await?
            } else {
                engine.validate(&value, Some(options.clone())).await?
            };

            if !report.valid {
                any_failures = true;
            }

            self.render_validation_report(data_path, &mut report, max_errors, show_stats)?;
        }

        if strict && any_failures {
            return Err(LinkMLError::DataValidationError {
                message: "Validation failed in strict mode".to_string(),
                path: None,
                expected: Some("valid data".to_string()),
                actual: Some("schema violations".to_string()),
            });
        }

        Ok(())
    }

    async fn generate_command(
        &self,
        schema_path: &Path,
        generator_name: &str,
        output_path: &Path,
        options: &[String],
    ) -> Result<()> {
        let schema = self.load_schema(schema_path).await?;
        let registry = GeneratorRegistry::with_defaults().await;

        let resolved_name = Self::resolve_generator_name(generator_name);
        let generator = registry.get(&resolved_name).await.ok_or_else(|| {
            LinkMLError::NotImplemented(format!("Generator '{generator_name}' is not registered"))
        })?;

        let generator_options = self.parse_generator_options(options)?;
        generator
            .validate_schema(&schema)
            .map_err(|err| LinkMLError::schema_validation(err.to_string()))?;
        let content = generator.generate(&schema)?;

        let target_file = self
            .prepare_output_path(output_path, generator.as_ref())
            .await?;
        fs::write(&target_file, content)
            .await
            .map_err(LinkMLError::from)?;

        if !self.cli.quiet {
            println!("Generated output: {}", target_file.display());
        }

        info!("Code generation completed using {}", generator.name());
        drop(generator_options);
        Ok(())
    }

    async fn convert_command(
        &self,
        input: &Path,
        output: &Path,
        from: Option<SchemaFormat>,
        to: SchemaFormat,
        pretty: bool,
        validate: bool,
    ) -> Result<()> {
        let input_format = from.unwrap_or_else(|| Self::detect_schema_format(input));
        let schema = self.read_schema_with_format(input, input_format).await?;

        if validate {
            self.basic_schema_sanity_check(&schema, input)?;
        }

        let serialized = match to {
            SchemaFormat::Yaml => serde_yaml::to_string(&schema)
                .map_err(|err| LinkMLError::SerializationError(err.to_string()))?,
            SchemaFormat::Json | SchemaFormat::JsonLd => {
                if pretty {
                    serde_json::to_string_pretty(&schema)?
                } else {
                    serde_json::to_string(&schema)?
                }
            }
        };

        if let Some(parent) = output.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent).await?;
        }

        fs::write(output, serialized).await?;

        if !self.cli.quiet {
            println!(
                "Converted schema {} -> {}",
                input.display(),
                output.display()
            );
        }

        Ok(())
    }

    async fn lint_command(
        &self,
        schema_path: &Path,
        rule_filters: &[String],
        config_path: Option<&PathBuf>,
        apply_fixes: bool,
        strict: bool,
        format: LintFormat,
    ) -> Result<()> {
        let mut options = LintOptions::default();
        if !rule_filters.is_empty() {
            options.filter_rules(rule_filters);
        }

        if let Some(config) = config_path {
            let config_content = fs::read_to_string(config).await.map_err(|err| {
                LinkMLError::DataValidationError {
                    message: format!("Failed to read lint config: {err}"),
                    path: Some(config.display().to_string()),
                    expected: Some("readable file".to_string()),
                    actual: Some("read error".to_string()),
                }
            })?;
            let parsed: HashMap<String, serde_json::Value> = serde_yaml::from_str(&config_content)
                .or_else(|_| serde_json::from_str(&config_content))
                .map_err(|err| LinkMLError::config(format!("Invalid lint config: {err}")))?;
            options.apply_config(parsed);
        }

        let schema = self.load_schema(schema_path).await?;
        let linter = SchemaLinter::new(options);
        let mut result = linter.lint(&schema)?;

        if apply_fixes {
            let mut mutable_schema = schema.clone();
            let fixed = linter.fix(&mut mutable_schema, &mut result)?;
            if fixed > 0 && !self.cli.quiet {
                println!("Applied {fixed} automatic fixes");
            }
        }

        let output = match format {
            LintFormat::Pretty => Self::render_lint_pretty(&result),
            LintFormat::Json => serde_json::to_string_pretty(&result)
                .map_err(|err| LinkMLError::SerializationError(err.to_string()))?,
            LintFormat::Github => Self::render_lint_github(&result),
            LintFormat::Junit => Self::render_lint_junit(&result),
        };

        self.print_output(&output);

        let has_errors = result
            .issues
            .iter()
            .any(|issue| issue.severity == Severity::Error);
        if strict && has_errors {
            return Err(LinkMLError::SchemaValidationError {
                message: "Linting detected errors".to_string(),
                element: Some(schema_path.display().to_string()),
            });
        }

        Ok(())
    }

    async fn diff_command(
        &self,
        schema1: &Path,
        schema2: &Path,
        format: DiffFormat,
        include_docs: bool,
        breaking_only: bool,
        context_lines: usize,
        output_path: Option<&PathBuf>,
    ) -> Result<()> {
        let first = self.load_schema(schema1).await?;
        let second = self.load_schema(schema2).await?;

        let options = DiffOptions {
            include_documentation: include_docs,
            breaking_changes_only: breaking_only,
            context_lines,
        };
        let differ = SchemaDiff::new(options);
        let diff = differ.diff(&first, &second)?;

        let rendered = match format {
            DiffFormat::Unified => self.render_diff_unified(schema1, schema2, &diff),
            DiffFormat::SideBySide => Self::render_diff_side_by_side(&diff),
            DiffFormat::JsonPatch => serde_json::to_string_pretty(&diff)
                .map_err(|err| LinkMLError::SerializationError(err.to_string()))?,
            DiffFormat::Html => Self::render_diff_html(&diff),
            DiffFormat::Markdown => Self::render_diff_markdown(&diff),
        };

        if let Some(path) = output_path {
            if let Some(parent) = path.parent()
                && !parent.as_os_str().is_empty()
            {
                fs::create_dir_all(parent).await?;
            }
            fs::write(path, &rendered).await?;
        } else {
            self.print_output(&rendered);
        }

        Ok(())
    }

    async fn merge_command(
        &self,
        schemas: &[PathBuf],
        output: &Path,
        strategy: MergeStrategy,
        conflict_resolution: ConflictResolution,
        base_schema: Option<&PathBuf>,
        validate: bool,
    ) -> Result<()> {
        if schemas.len() < 2 {
            return Err(LinkMLError::config(
                "Merge command requires at least two schema inputs",
            ));
        }

        let mut loaded = Vec::with_capacity(schemas.len());
        for schema_path in schemas {
            loaded.push(self.load_schema(schema_path).await?);
        }

        let base = if let Some(path) = base_schema {
            Some(self.load_schema(path).await?)
        } else {
            None
        };

        let merge_options = MergeOptions {
            strategy,
            conflict_resolution,
            base_schema: base,
            preserve_annotations: true,
            merge_imports: true,
        };

        let merge_engine = SchemaMerge::new(merge_options);
        let merged = merge_engine.merge(&loaded)?;

        if validate {
            self.basic_schema_sanity_check(&merged, output)?;
        }

        let serialized = serde_yaml::to_string(&merged)
            .map_err(|err| LinkMLError::SerializationError(err.to_string()))?;

        if let Some(parent) = output.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent).await?;
        }

        fs::write(output, serialized).await?;

        if !self.cli.quiet {
            println!("Merged {} schemas into {}", schemas.len(), output.display());
        }

        Ok(())
    }

    async fn load_command(
        &self,
        schema_path: &Path,
        input_path: &Path,
        format: LoadFormat,
        output_path: &Path,
        options: &[String],
        validate: bool,
        class_name: Option<&str>,
    ) -> Result<()> {
        let schema = self.load_schema(schema_path).await?;

        // Parse load options
        let mut load_options = std::collections::HashMap::new();
        for option in options {
            if let Some((key, value)) = option.split_once('=') {
                load_options.insert(key.trim().to_string(), value.trim().to_string());
            } else {
                return Err(LinkMLError::config(format!(
                    "Invalid option format: '{}'. Expected 'key=value' format.",
                    option
                )));
            }
        }

        // Load data based on format
        let loaded_data = match format {
            LoadFormat::Json => {
                let content = fs::read_to_string(input_path).await?;
                serde_json::from_str::<serde_json::Value>(&content)
                    .map_err(|e| LinkMLError::data_validation(format!("JSON parse error: {e}")))?
            }
            LoadFormat::Yaml => {
                let content = fs::read_to_string(input_path).await?;
                serde_yaml::from_str::<serde_json::Value>(&content)
                    .map_err(|e| LinkMLError::data_validation(format!("YAML parse error: {e}")))?
            }
            LoadFormat::Csv => {
                let content = fs::read_to_string(input_path).await?;
                self.load_csv_data(&content, &load_options)?
            }
            LoadFormat::Xml => {
                return Err(LinkMLError::not_implemented(
                    "XML loading requires integration with parse-service XML parser",
                ));
            }
            LoadFormat::Rdf => {
                return Err(LinkMLError::not_implemented(
                    "RDF loading requires integration with graph-database-service",
                ));
            }
            LoadFormat::Database => {
                return Err(LinkMLError::not_implemented(
                    "Database loading requires integration with dbms-service",
                ));
            }
            LoadFormat::Api => {
                return Err(LinkMLError::not_implemented(
                    "API loading requires integration with external-api-service",
                ));
            }
            LoadFormat::TypeDb => {
                return Err(LinkMLError::not_implemented(
                    "TypeDB loading requires integration with graph-database-service TypeDB backend",
                ));
            }
        };

        // Validate loaded data if requested
        if validate {
            let engine = ValidationEngine::new(&schema)
                .map_err(|err| LinkMLError::service(format!("Failed to build validator: {err}")))?;

            let validation_options = ValidationOptions {
                fail_fast: Some(false),
                parallel: Some(false),
                allow_additional_properties: None,
                max_depth: None,
                check_permissibles: None,
                use_cache: Some(true),
                fail_on_warning: None,
                custom_validators: Vec::new(),
            };

            let report = if let Some(target_class) = class_name {
                engine
                    .validate_as_class(&loaded_data, target_class, Some(validation_options))
                    .await?
            } else {
                engine
                    .validate(&loaded_data, Some(validation_options))
                    .await?
            };

            if !report.valid {
                return Err(LinkMLError::DataValidationError {
                    message: format!(
                        "Loaded data failed validation with {} errors",
                        report.issues.len()
                    ),
                    path: Some(input_path.display().to_string()),
                    expected: Some("valid data according to schema".to_string()),
                    actual: Some(format!("{} validation errors", report.issues.len())),
                });
            }
        }

        // Convert to LinkML canonical format and save
        let output_content = match output_path.extension().and_then(|e| e.to_str()) {
            Some("json") => serde_json::to_string_pretty(&loaded_data)?,
            Some("yaml" | "yml") => serde_yaml::to_string(&loaded_data)
                .map_err(|e| LinkMLError::SerializationError(e.to_string()))?,
            _ => serde_json::to_string_pretty(&loaded_data)?,
        };

        // Ensure output directory exists
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::write(output_path, output_content).await?;

        if !self.cli.quiet {
            println!(
                "Loaded {} data from {} to {}",
                format!("{format:?}").to_lowercase(),
                input_path.display(),
                output_path.display()
            );
        }

        Ok(())
    }

    async fn dump_command(
        &self,
        schema_path: &Path,
        input_path: &Path,
        output_path: &Path,
        format: DumpFormat,
        options: &[String],
        pretty: bool,
    ) -> Result<()> {
        let _schema = self.load_schema(schema_path).await?;
        let input_data = self.load_data_value(input_path).await?;

        // Parse dump options
        let mut dump_options = std::collections::HashMap::new();
        for option in options {
            if let Some((key, value)) = option.split_once('=') {
                dump_options.insert(key.trim().to_string(), value.trim().to_string());
            } else {
                return Err(LinkMLError::config(format!(
                    "Invalid option format: '{}'. Expected 'key=value' format.",
                    option
                )));
            }
        }

        // Dump data based on format
        let output_content = match format {
            DumpFormat::Json => {
                if pretty {
                    serde_json::to_string_pretty(&input_data)?
                } else {
                    serde_json::to_string(&input_data)?
                }
            }
            DumpFormat::Yaml => serde_yaml::to_string(&input_data)
                .map_err(|e| LinkMLError::SerializationError(e.to_string()))?,
            DumpFormat::Csv => self.dump_csv_data(&input_data, &dump_options)?,
            DumpFormat::Xml => {
                return Err(LinkMLError::not_implemented(
                    "XML dumping requires integration with parse-service XML generator",
                ));
            }
            DumpFormat::Rdf => {
                return Err(LinkMLError::not_implemented(
                    "RDF dumping requires integration with graph-database-service",
                ));
            }
            DumpFormat::Database => {
                return Err(LinkMLError::not_implemented(
                    "Database dumping requires integration with dbms-service",
                ));
            }
            DumpFormat::Api => {
                return Err(LinkMLError::not_implemented(
                    "API dumping requires integration with external-api-service",
                ));
            }
            DumpFormat::TypeDb => {
                return Err(LinkMLError::not_implemented(
                    "TypeDB dumping requires integration with graph-database-service TypeDB backend",
                ));
            }
        };

        // Ensure output directory exists
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::write(output_path, output_content).await?;

        if !self.cli.quiet {
            println!(
                "Dumped data to {} format: {}",
                format!("{format:?}").to_lowercase(),
                output_path.display()
            );
        }

        Ok(())
    }

    async fn serve_command(&self, schema: &Path, port: u16, host: &str) -> Result<()> {
        let command = ServeCommand::new(schema.display().to_string(), port)
            .with_host(host.to_string())
            .with_verbose(self.cli.verbose);
        command.execute().await
    }

    async fn sheets2schema_command(
        &self,
        input: &Path,
        output: Option<&PathBuf>,
        schema_id: Option<&String>,
        schema_name: Option<&String>,
        format: SchemaFormat,
        progress: bool,
    ) -> Result<()> {
        use crate::cli_enhanced::commands::sheets2schema::{
            SchemaFormat as CmdSchemaFormat, Sheets2SchemaCommand,
        };

        let mut command = Sheets2SchemaCommand::new(input.to_path_buf(), output.cloned())
            .with_progress(progress && !self.cli.quiet)
            .with_verbose(self.cli.verbose);

        if let Some(id) = schema_id {
            command = command.with_schema_id(id.clone());
        }

        if let Some(name) = schema_name {
            command = command.with_schema_name(name.clone());
        }

        let cmd_format = match format {
            SchemaFormat::Yaml => CmdSchemaFormat::Yaml,
            SchemaFormat::Json => CmdSchemaFormat::Json,
            _ => CmdSchemaFormat::Yaml,
        };
        command = command.with_format(cmd_format);

        command.execute().await
    }

    async fn schema2sheets_command(
        &self,
        schema: &Path,
        output: &Path,
        validation: bool,
        examples: bool,
        freeze_headers: bool,
        filters: bool,
        progress: bool,
    ) -> Result<()> {
        use crate::cli_enhanced::commands::schema2sheets::Schema2SheetsCommand;

        let command = Schema2SheetsCommand::new(schema.to_path_buf(), output.to_path_buf())
            .with_validation(validation)
            .with_examples(examples)
            .with_freeze_headers(freeze_headers)
            .with_filters(filters)
            .with_progress(progress && !self.cli.quiet)
            .with_verbose(self.cli.verbose);

        command.execute().await
    }

    async fn load_schema(&self, path: &Path) -> Result<SchemaDefinition> {
        let format = Self::detect_schema_format(path);
        self.read_schema_with_format(path, format).await
    }

    async fn read_schema_with_format(
        &self,
        path: &Path,
        format: SchemaFormat,
    ) -> Result<SchemaDefinition> {
        let content =
            fs::read_to_string(path)
                .await
                .map_err(|err| LinkMLError::DataValidationError {
                    message: format!("Failed to read schema file: {err}"),
                    path: Some(path.display().to_string()),
                    expected: Some("readable file".to_string()),
                    actual: Some("read error".to_string()),
                })?;

        let schema = match format {
            SchemaFormat::Yaml => {
                serde_yaml::from_str(&content).map_err(|err| LinkMLError::ParseError {
                    message: err.to_string(),
                    location: Some(path.display().to_string()),
                })?
            }
            SchemaFormat::Json | SchemaFormat::JsonLd => {
                serde_json::from_str(&content).map_err(|err| LinkMLError::ParseError {
                    message: err.to_string(),
                    location: Some(path.display().to_string()),
                })?
            }
        };

        Ok(schema)
    }

    async fn load_data_value(&self, path: &Path) -> Result<Value> {
        let content =
            fs::read_to_string(path)
                .await
                .map_err(|err| LinkMLError::DataValidationError {
                    message: format!("Failed to read data file: {err}"),
                    path: Some(path.display().to_string()),
                    expected: Some("readable file".to_string()),
                    actual: Some("read error".to_string()),
                })?;

        if matches!(
            path.extension().and_then(|ext| ext.to_str()),
            Some("yaml" | "yml")
        ) {
            serde_yaml::from_str(&content)
                .map_err(|err| LinkMLError::data_validation(format!("YAML parse error: {err}")))
        } else {
            serde_json::from_str(&content)
                .map_err(|err| LinkMLError::data_validation(format!("JSON parse error: {err}")))
        }
    }

    fn basic_schema_sanity_check(&self, schema: &SchemaDefinition, source: &Path) -> Result<()> {
        if schema.name.trim().is_empty() {
            return Err(LinkMLError::schema_validation(format!(
                "Schema at {} is missing a name",
                source.display()
            )));
        }
        if schema.classes.is_empty() {
            warn!("Schema {} does not define any classes", source.display());
        }
        Ok(())
    }

    fn detect_schema_format(path: &Path) -> SchemaFormat {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("json" | "jsonld") => SchemaFormat::Json,
            _ => SchemaFormat::Yaml,
        }
    }

    fn resolve_generator_name(raw: &str) -> String {
        match raw.to_ascii_lowercase().as_str() {
            "python" | "py" => "python-dataclass".to_string(),
            "pydantic" => "pydantic".to_string(),
            "rust" => "rust".to_string(),
            "typescript" | "ts" => "typescript".to_string(),
            other => other.to_string(),
        }
    }

    async fn prepare_output_path(
        &self,
        output_path: &Path,
        generator: &dyn Generator,
    ) -> Result<PathBuf> {
        if let Ok(metadata) = fs::metadata(output_path).await
            && metadata.is_dir()
        {
            let target = output_path.join(generator.get_default_filename());
            return Ok(target);
        }

        if output_path.extension().is_some() {
            if let Some(parent) = output_path.parent()
                && !parent.as_os_str().is_empty()
            {
                fs::create_dir_all(parent).await?;
            }
            Ok(output_path.to_path_buf())
        } else {
            fs::create_dir_all(output_path).await?;
            Ok(output_path.join(generator.get_default_filename()))
        }
    }

    fn parse_generator_options(&self, options: &[String]) -> Result<GeneratorOptions> {
        let mut generator_options = GeneratorOptions::default();

        // Parse key=value pairs from options
        for option in options {
            if let Some((key, value)) = option.split_once('=') {
                match key.trim() {
                    "indent" => {
                        if let Ok(indent) = value.trim().parse::<usize>() {
                            generator_options.indent = IndentStyle::Spaces(indent);
                        } else {
                            return Err(LinkMLError::config(format!(
                                "Invalid indent value: '{}'. Expected integer.",
                                value.trim()
                            )));
                        }
                    }
                    "pretty" => match value.trim().to_lowercase().as_str() {
                        "true" | "yes" | "1" => {
                            generator_options
                                .custom
                                .insert("pretty_print".to_string(), "true".to_string());
                        }
                        "false" | "no" | "0" => {
                            generator_options
                                .custom
                                .insert("pretty_print".to_string(), "false".to_string());
                        }
                        _ => {
                            return Err(LinkMLError::config(format!(
                                "Invalid pretty value: '{}'. Expected boolean.",
                                value.trim()
                            )));
                        }
                    },
                    "include_docs" => match value.trim().to_lowercase().as_str() {
                        "true" | "yes" | "1" => generator_options.include_docs = true,
                        "false" | "no" | "0" => generator_options.include_docs = false,
                        _ => {
                            return Err(LinkMLError::config(format!(
                                "Invalid include_docs value: '{}'. Expected boolean.",
                                value.trim()
                            )));
                        }
                    },
                    "namespace" => {
                        let namespace = value.trim().to_string();
                        if namespace.is_empty() {
                            return Err(LinkMLError::config(
                                "Namespace value cannot be empty".to_string(),
                            ));
                        }
                        generator_options
                            .custom
                            .insert("namespace".to_string(), namespace);
                    }
                    "package" => {
                        let package = value.trim().to_string();
                        if package.is_empty() {
                            return Err(LinkMLError::config(
                                "Package value cannot be empty".to_string(),
                            ));
                        }
                        generator_options
                            .custom
                            .insert("package_name".to_string(), package);
                    }
                    unknown_key => {
                        return Err(LinkMLError::config(format!(
                            "Unknown generator option: '{}'. Supported options: indent, pretty, include_docs, namespace, package",
                            unknown_key
                        )));
                    }
                }
            } else {
                return Err(LinkMLError::config(format!(
                    "Invalid option format: '{}'. Expected 'key=value' format.",
                    option
                )));
            }
        }

        Ok(generator_options)
    }

    fn load_csv_data(
        &self,
        csv_content: &str,
        options: &std::collections::HashMap<String, String>,
    ) -> Result<serde_json::Value> {
        let delimiter = options
            .get("delimiter")
            .map(|d| d.chars().next().unwrap_or(','))
            .unwrap_or(',');
        let has_headers = options
            .get("headers")
            .map(|h| matches!(h.to_lowercase().as_str(), "true" | "yes" | "1"))
            .unwrap_or(true);

        let mut reader = csv::ReaderBuilder::new()
            .delimiter(u8::try_from(delimiter as u32).unwrap_or(b','))
            .has_headers(has_headers)
            .from_reader(csv_content.as_bytes());

        let headers = if has_headers {
            reader
                .headers()
                .map_err(|e| LinkMLError::data_validation(format!("CSV header error: {e}")))?
                .iter()
                .map(String::from)
                .collect::<Vec<_>>()
        } else {
            // Generate column names
            let first_record = reader
                .records()
                .next()
                .ok_or_else(|| LinkMLError::data_validation("Empty CSV file".to_string()))?
                .map_err(|e| LinkMLError::data_validation(format!("CSV parse error: {e}")))?;
            (0..first_record.len())
                .map(|i| format!("column_{i}"))
                .collect::<Vec<_>>()
        };

        let mut records = Vec::new();
        for result in reader.records() {
            let record = result
                .map_err(|e| LinkMLError::data_validation(format!("CSV record error: {e}")))?;
            let mut row_data = serde_json::Map::new();

            for (i, field) in record.iter().enumerate() {
                if let Some(header) = headers.get(i) {
                    // Try to parse as number first, then boolean, then string
                    let value = if let Ok(num) = field.parse::<i64>() {
                        serde_json::Value::Number(serde_json::Number::from(num))
                    } else if let Ok(num) = field.parse::<f64>() {
                        serde_json::Number::from_f64(num).map_or_else(
                            || serde_json::Value::String(field.to_string()),
                            serde_json::Value::Number,
                        )
                    } else {
                        serde_json::Value::String(field.to_string())
                    };
                    row_data.insert(header.clone(), value);
                }
            }
            records.push(serde_json::Value::Object(row_data));
        }

        Ok(serde_json::Value::Array(records))
    }

    fn dump_csv_data(
        &self,
        data: &serde_json::Value,
        options: &std::collections::HashMap<String, String>,
    ) -> Result<String> {
        let delimiter = options
            .get("delimiter")
            .map(|d| d.chars().next().unwrap_or(','))
            .unwrap_or(',');
        let include_headers = options
            .get("headers")
            .map(|h| matches!(h.to_lowercase().as_str(), "true" | "yes" | "1"))
            .unwrap_or(true);

        let mut output = Vec::new();
        let mut writer = csv::WriterBuilder::new()
            .delimiter(u8::try_from(delimiter as u32).unwrap_or(b','))
            .has_headers(include_headers)
            .from_writer(&mut output);

        match data {
            serde_json::Value::Array(records) => {
                if records.is_empty() {
                    return Ok(String::new());
                }

                // Extract headers from first record
                let headers = if let Some(serde_json::Value::Object(first_record)) = records.first()
                {
                    first_record.keys().cloned().collect::<Vec<_>>()
                } else {
                    return Err(LinkMLError::data_validation(
                        "CSV export requires array of objects".to_string(),
                    ));
                };

                // Write headers if enabled
                if include_headers {
                    writer
                        .write_record(&headers)
                        .map_err(|e| LinkMLError::SerializationError(e.to_string()))?;
                }

                // Write records
                for record in records {
                    if let serde_json::Value::Object(obj) = record {
                        let mut row = Vec::new();
                        for header in &headers {
                            let value = obj.get(header).unwrap_or(&serde_json::Value::Null);
                            let str_value = match value {
                                serde_json::Value::String(s) => s.clone(),
                                serde_json::Value::Number(n) => n.to_string(),
                                serde_json::Value::Bool(b) => b.to_string(),
                                serde_json::Value::Null => String::new(),
                                _ => serde_json::to_string(value)
                                    .map_err(|e| LinkMLError::SerializationError(e.to_string()))?,
                            };
                            row.push(str_value);
                        }
                        writer
                            .write_record(&row)
                            .map_err(|e| LinkMLError::SerializationError(e.to_string()))?;
                    } else {
                        return Err(LinkMLError::data_validation(
                            "CSV export requires array of objects".to_string(),
                        ));
                    }
                }
            }
            serde_json::Value::Object(obj) => {
                // Single object - export as one row
                let headers: Vec<String> = obj.keys().cloned().collect();

                if include_headers {
                    writer
                        .write_record(&headers)
                        .map_err(|e| LinkMLError::SerializationError(e.to_string()))?;
                }

                let mut row = Vec::new();
                for header in &headers {
                    let value = obj.get(header).unwrap_or(&serde_json::Value::Null);
                    let str_value = match value {
                        serde_json::Value::String(s) => s.clone(),
                        serde_json::Value::Number(n) => n.to_string(),
                        serde_json::Value::Bool(b) => b.to_string(),
                        serde_json::Value::Null => String::new(),
                        _ => serde_json::to_string(value)
                            .map_err(|e| LinkMLError::SerializationError(e.to_string()))?,
                    };
                    row.push(str_value);
                }
                writer
                    .write_record(&row)
                    .map_err(|e| LinkMLError::SerializationError(e.to_string()))?;
            }
            _ => {
                return Err(LinkMLError::data_validation(
                    "CSV export requires object or array of objects".to_string(),
                ));
            }
        }

        writer
            .flush()
            .map_err(|e| LinkMLError::SerializationError(e.to_string()))?;

        // Drop the writer to release the borrow on output
        drop(writer);

        String::from_utf8(output)
            .map_err(|e| LinkMLError::SerializationError(format!("UTF-8 conversion error: {e}")))
    }

    fn render_validation_report(
        &self,
        data_path: &Path,
        report: &mut ValidationReport,
        max_errors: usize,
        show_stats: bool,
    ) -> Result<()> {
        report.sort_issues();
        let mut buffer = String::new();
        writeln!(&mut buffer, "{}", data_path.display())
            .map_err(|e| LinkMLError::service(format!("Failed to write to buffer: {e}")))?;
        writeln!(&mut buffer, "{}", report.summary())
            .map_err(|e| LinkMLError::service(format!("Failed to write to buffer: {e}")))?;

        if !report.issues.is_empty() {
            writeln!(&mut buffer, "Issues:")
                .map_err(|e| LinkMLError::service(format!("Failed to write to buffer: {e}")))?;
            for issue in report.issues.iter().take(max_errors.max(1)) {
                writeln!(&mut buffer, "  {issue}")
                    .map_err(|e| LinkMLError::service(format!("Failed to write to buffer: {e}")))?;
            }
            if report.issues.len() > max_errors {
                writeln!(
                    &mut buffer,
                    "  … {} additional issues suppressed",
                    report.issues.len() - max_errors
                )
                .map_err(|e| LinkMLError::service(format!("Failed to write to buffer: {e}")))?;
            }
        }

        if show_stats {
            writeln!(
                &mut buffer,
                "Stats: errors={}, warnings={}, duration={}ms",
                report.stats.error_count, report.stats.warning_count, report.stats.duration_ms
            )
            .map_err(|e| LinkMLError::service(format!("Failed to write to buffer: {e}")))?;
        }

        self.print_output(&buffer);
        Ok(())
    }

    fn render_lint_pretty(result: &crate::schema::LintResult) -> String {
        let mut buffer = String::new();
        let _ = writeln!(
            &mut buffer,
            "Lint completed with {} issues ({} errors, {} warnings)",
            result.issues.len(),
            result.error_count(),
            result.warning_count()
        );

        for issue in &result.issues {
            let _ = writeln!(
                &mut buffer,
                "- [{:?}] {}: {}",
                issue.severity,
                issue.element_name.as_deref().unwrap_or("<unknown>"),
                issue.message
            );
        }

        buffer
    }

    fn render_lint_github(result: &crate::schema::LintResult) -> String {
        let mut buffer = String::new();
        for issue in &result.issues {
            let severity = match issue.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
                Severity::Info => "notice",
            };
            let line = issue.line.unwrap_or(1);
            let column = issue.column.unwrap_or(1);
            let _ = writeln!(
                &mut buffer,
                "::{} file={},line={},col={}::{}",
                severity,
                issue.element_name.as_deref().unwrap_or("schema"),
                line,
                column,
                issue.message
            );
        }
        buffer
    }

    fn render_lint_junit(result: &crate::schema::LintResult) -> String {
        let mut buffer = String::new();
        buffer.push_str("<testsuites><testsuite name=\"linkml-lint\">");
        for issue in &result.issues {
            let status = if issue.severity == Severity::Error {
                "failed"
            } else {
                "passed"
            };
            let name = issue.element_name.as_deref().unwrap_or("schema");
            if status == "failed" {
                buffer.push_str(&format!(
                    "<testcase name=\"{}\" status=\"failed\"><failure>{}</failure></testcase>",
                    name, issue.message
                ));
            } else {
                buffer.push_str(&format!("<testcase name=\"{name}\" status=\"passed\"/>"));
            }
        }
        buffer.push_str("</testsuite></testsuites>");
        buffer
    }

    fn render_diff_unified(
        &self,
        schema1: &Path,
        schema2: &Path,
        diff: &crate::schema::DiffResult,
    ) -> String {
        let mut buffer = String::new();
        let _ = writeln!(
            &mut buffer,
            "--- {}\n+++ {}",
            schema1.display(),
            schema2.display()
        );

        for class in &diff.removed_classes {
            let _ = writeln!(&mut buffer, "- class {class}");
        }
        for class in &diff.added_classes {
            let _ = writeln!(&mut buffer, "+ class {class}");
        }
        for slot in &diff.removed_slots {
            let _ = writeln!(&mut buffer, "- slot {slot}");
        }
        for slot in &diff.added_slots {
            let _ = writeln!(&mut buffer, "+ slot {slot}");
        }
        buffer
    }

    fn render_diff_side_by_side(diff: &crate::schema::DiffResult) -> String {
        let mut buffer = String::new();
        let _ = writeln!(&mut buffer, "Left | Right");
        let _ = writeln!(&mut buffer, "---- | ----");
        for removed in &diff.removed_classes {
            let _ = writeln!(&mut buffer, "{removed} |");
        }
        for added in &diff.added_classes {
            let _ = writeln!(&mut buffer, " | {added}");
        }
        buffer
    }

    fn render_diff_html(diff: &crate::schema::DiffResult) -> String {
        let mut buffer = String::new();
        buffer.push_str("<html><body><h1>Schema Diff</h1>");
        buffer.push_str("<h2>Added Classes</h2><ul>");
        for class in &diff.added_classes {
            buffer.push_str(&format!("<li>{class}</li>"));
        }
        buffer.push_str("</ul><h2>Removed Classes</h2><ul>");
        for class in &diff.removed_classes {
            buffer.push_str(&format!("<li>{class}</li>"));
        }
        buffer.push_str("</ul></body></html>");
        buffer
    }

    fn render_diff_markdown(diff: &crate::schema::DiffResult) -> String {
        let mut buffer = String::new();
        buffer.push_str("# Schema Diff\n\n");
        buffer.push_str("## Added Classes\n");
        for class in &diff.added_classes {
            buffer.push_str(&format!("- {class}\n"));
        }
        buffer.push_str("\n## Removed Classes\n");
        for class in &diff.removed_classes {
            buffer.push_str(&format!("- {class}\n"));
        }
        buffer
    }

    fn format_output(&self, body: &str) -> String {
        let timestamp = self
            .timestamp_utils
            .now_rfc3339()
            .unwrap_or_else(|_| "unknown".to_string());
        match self.cli.format {
            OutputFormat::Pretty => body.to_string(),
            OutputFormat::Json => serde_json::json!({
                "timestamp": timestamp,
                "output": body,
            })
            .to_string(),
            OutputFormat::Yaml => format!(
                "timestamp: {timestamp}\noutput: |\n  {}",
                body.replace('\n', "\n  ")
            ),
            OutputFormat::Tsv => format!(
                "timestamp\toutput\n{timestamp}\t{}",
                body.replace('\n', " ")
            ),
            OutputFormat::Minimal => body.lines().next().unwrap_or("").to_string(),
        }
    }

    fn print_output(&self, body: &str) {
        let rendered = self.format_output(body);
        println!("{rendered}");
    }
}

// Note: No Default implementation - proper dependency injection requires
// explicit service provisioning via from_args_with_timestamp() or new()
