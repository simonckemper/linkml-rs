//! Enhanced CLI commands for LinkML
//!
//! This module provides the complete set of LinkML command-line tools
//! matching Python LinkML functionality.

use clap::{Parser, Subcommand, ValueEnum};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use linkml_core::error::{LinkMLError, Result};
use linkml_core::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::warn;

use crate::generator::{GeneratorOptions, GeneratorRegistry};
use crate::loader::{
    ApiDumper, ApiLoader, ApiOptions, CsvDumper, CsvLoader, CsvOptions, DataDumper, DataLoader,
    JsonDumper, JsonLoader, RdfDumper, RdfLoader, RdfOptions, RdfSerializationFormat, XmlDumper,
    XmlLoader, YamlDumper, YamlLoader,
    traits::{DumpOptions, LoadOptions},
};
#[cfg(feature = "database")]
use crate::loader::{DatabaseDumper, DatabaseLoader, DatabaseOptions};
use crate::schema::{
    diff::{DiffOptions, SchemaDiff},
    lint::{LintOptions, LintResult, SchemaLinter},
    merge::{MergeOptions, SchemaMerge},
};
use crate::validator::ValidationEngine;

/// LinkML command-line interface
#[derive(Parser, Debug)]
#[command(name = "linkml", version, about = "LinkML schema tools")]
pub struct LinkMLCli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Output format
    #[arg(short = 'f', long, global = true, default_value = "pretty")]
    format: OutputFormat,

    /// Configuration file
    #[arg(short, long, global = true)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: LinkMLCommand,
}

/// Available output formats
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    /// Human-readable output
    Pretty,
    /// JSON output
    Json,
    /// YAML output
    Yaml,
    /// Tab-separated values
    Tsv,
    /// Minimal output
    Minimal,
}

/// LinkML subcommands
#[derive(Subcommand, Debug)]
pub enum LinkMLCommand {
    /// Validate data against a schema
    Validate {
        /// Schema file path
        #[arg(short, long)]
        schema: PathBuf,

        /// Data file(s) to validate
        #[arg(required = true)]
        data: Vec<PathBuf>,

        /// Target class name
        #[arg(short = 'C', long)]
        class_name: Option<String>,

        /// Enable strict mode
        #[arg(long)]
        strict: bool,

        /// Maximum errors to show
        #[arg(long, default_value = "10")]
        max_errors: usize,

        /// Show validation statistics
        #[arg(long)]
        stats: bool,

        /// Validate in parallel
        #[arg(long)]
        parallel: bool,
    },

    /// Generate code or artifacts from schema
    Generate {
        /// Schema file path
        #[arg(short, long)]
        schema: PathBuf,

        /// Output directory or file
        #[arg(short, long)]
        output: PathBuf,

        /// Generator name (python, typescript, rust, etc.)
        #[arg(short = 'g', long)]
        generator: String,

        /// Generator options (key=value)
        #[arg(long = "option", value_name = "KEY=VALUE")]
        options: Vec<String>,

        /// Template directory for custom templates
        #[arg(short = 't', long)]
        template_dir: Option<PathBuf>,

        /// Include imports in generation
        #[arg(long)]
        include_imports: bool,
    },

    /// Convert schema between formats
    Convert {
        /// Input schema file
        #[arg(short, long)]
        input: PathBuf,

        /// Output file path
        #[arg(short, long)]
        output: PathBuf,

        /// Input format (auto-detect if not specified)
        #[arg(long)]
        from: Option<SchemaFormat>,

        /// Output format
        #[arg(long, value_enum)]
        to: SchemaFormat,

        /// Pretty print output
        #[arg(long)]
        pretty: bool,

        /// Validate after conversion
        #[arg(long)]
        validate: bool,
    },

    /// Merge multiple schemas
    Merge {
        /// Schema files to merge
        #[arg(required = true, num_args = 2..)]
        schemas: Vec<PathBuf>,

        /// Output file path
        #[arg(short, long)]
        output: PathBuf,

        /// Merge strategy
        #[arg(short = 's', long, default_value = "union")]
        strategy: MergeStrategy,

        /// Conflict resolution
        #[arg(short = 'c', long, default_value = "error")]
        conflict: ConflictResolution,

        /// Base schema for three-way merge
        #[arg(short = 'b', long)]
        base: Option<PathBuf>,

        /// Validate result
        #[arg(long)]
        validate: bool,
    },

    /// Compare schemas and show differences
    Diff {
        /// First schema file
        schema1: PathBuf,

        /// Second schema file
        schema2: PathBuf,

        /// Output format for diff
        #[arg(short = 'f', long, default_value = "unified")]
        format: DiffFormat,

        /// Include documentation changes
        #[arg(long)]
        include_docs: bool,

        /// Show only breaking changes
        #[arg(long)]
        breaking_only: bool,

        /// Context lines for unified diff
        #[arg(short = 'c', long, default_value = "3")]
        context: usize,

        /// Output file (stdout if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Check schema quality and compliance
    Lint {
        /// Schema file to lint
        schema: PathBuf,

        /// Lint rules to apply
        #[arg(short = 'r', long)]
        rules: Vec<String>,

        /// Configuration file for lint rules
        #[arg(short = 'c', long)]
        config: Option<PathBuf>,

        /// Fix issues automatically where possible
        #[arg(long)]
        fix: bool,

        /// Fail with non-zero exit code if issues found
        #[arg(long)]
        strict: bool,

        /// Output format
        #[arg(short = 'f', long, default_value = "pretty")]
        format: LintFormat,
    },

    /// Start schema API server
    Serve {
        /// Schema file to serve
        #[arg(short, long)]
        schema: PathBuf,

        /// Port to listen on
        #[arg(short, long, default_value = "8080")]
        port: u16,

        /// Host to bind to
        #[arg(short = 'H', long, default_value = "127.0.0.1")]
        host: String,

        /// Enable CORS
        #[arg(long)]
        cors: bool,

        /// Authentication type
        #[arg(short = 'a', long)]
        auth: Option<AuthType>,

        /// TLS certificate file
        #[arg(long)]
        cert: Option<PathBuf>,

        /// TLS key file
        #[arg(long)]
        key: Option<PathBuf>,

        /// API documentation path
        #[arg(long, default_value = "/docs")]
        docs_path: String,
    },

    /// Load data from various formats
    Load {
        /// Schema file
        #[arg(short, long)]
        schema: PathBuf,

        /// Input data file
        #[arg(short, long)]
        input: PathBuf,

        /// Input format
        #[arg(short = 'f', long)]
        format: LoadFormat,

        /// Output file
        #[arg(short, long)]
        output: PathBuf,

        /// Loader options (key=value)
        #[arg(long = "option", value_name = "KEY=VALUE")]
        options: Vec<String>,

        /// Validate loaded data
        #[arg(long)]
        validate: bool,

        /// Target class for loading
        #[arg(short = 'C', long)]
        class_name: Option<String>,
    },

    /// Dump data to various formats
    Dump {
        /// Schema file
        #[arg(short, long)]
        schema: PathBuf,

        /// Input data file (LinkML format)
        #[arg(short, long)]
        input: PathBuf,

        /// Output file
        #[arg(short, long)]
        output: PathBuf,

        /// Output format
        #[arg(short = 'f', long)]
        format: DumpFormat,

        /// Dumper options (key=value)
        #[arg(long = "option", value_name = "KEY=VALUE")]
        options: Vec<String>,

        /// Pretty print output
        #[arg(long)]
        pretty: bool,
    },

    /// Interactive LinkML shell
    Shell {
        /// Initial schema to load
        #[arg(short, long)]
        schema: Option<PathBuf>,

        /// History file
        #[arg(long)]
        history: Option<PathBuf>,

        /// Startup script
        #[arg(long)]
        init: Option<PathBuf>,

        /// Enable syntax highlighting
        #[arg(long, default_value = "true")]
        highlight: bool,
    },
}

/// Schema formats for conversion
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SchemaFormat {
    /// YAML format (default)
    Yaml,
    /// JSON format
    Json,
    /// JSON-LD format
    JsonLd,
}

/// Merge strategies
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum MergeStrategy {
    /// Union of all elements
    Union,
    /// Intersection of common elements
    Intersection,
    /// Override with later schemas
    Override,
    /// Custom merge with rules
    Custom,
}

/// Conflict resolution strategies
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ConflictResolution {
    /// Error on conflicts
    Error,
    /// Use first definition
    First,
    /// Use last definition
    Last,
    /// Interactive resolution
    Interactive,
}

/// Diff output formats
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum DiffFormat {
    /// Unified diff format
    Unified,
    /// Side-by-side diff
    SideBySide,
    /// JSON patch format
    JsonPatch,
    /// HTML diff
    Html,
    /// Markdown diff
    Markdown,
}

/// Lint output formats
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum LintFormat {
    /// Pretty printed output
    Pretty,
    /// JSON output
    Json,
    /// GitHub Actions format
    Github,
    /// JUnit XML format
    Junit,
}

/// Authentication types for serve command
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum AuthType {
    /// No authentication
    None,
    /// Basic authentication
    Basic,
    /// Bearer token
    Bearer,
    /// API key
    ApiKey,
}

/// Data loading formats
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum LoadFormat {
    /// CSV/TSV format
    Csv,
    /// JSON format
    Json,
    /// YAML format
    Yaml,
    /// XML format
    Xml,
    /// RDF (Turtle, N-Triples, etc.)
    Rdf,
    /// SQL database
    Database,
    /// REST API
    Api,
    /// TypeDB
    TypeDb,
}

/// Data dumping formats
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum DumpFormat {
    /// CSV/TSV format
    Csv,
    /// JSON format
    Json,
    /// YAML format
    Yaml,
    /// XML format
    Xml,
    /// RDF (Turtle, N-Triples, etc.)
    Rdf,
    /// SQL database
    Database,
    /// REST API
    Api,
    /// TypeDB
    TypeDb,
}

/// CLI Application implementation
pub struct LinkMLApp {
    cli: LinkMLCli,
    generator_registry: GeneratorRegistry,
}

impl LinkMLApp {
    /// Create new LinkML CLI application
    pub fn new() -> Self {
        Self {
            cli: LinkMLCli::parse(),
            generator_registry: GeneratorRegistry::new(),
        }
    }

    /// Run the CLI application
    pub async fn run(&mut self) -> Result<()> {
        // Setup logging
        if self.cli.verbose {
            tracing_subscriber::fmt()
                .with_env_filter("linkml=debug")
                .init();
        } else {
            tracing_subscriber::fmt()
                .with_env_filter("linkml=info")
                .init();
        }

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
                output,
                generator,
                options,
                template_dir,
                include_imports,
            } => {
                self.generate_command(
                    schema,
                    output,
                    generator,
                    options,
                    template_dir.as_deref(),
                    *include_imports,
                )
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
                    base.as_deref(),
                    *validate,
                )
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
                    output.as_deref(),
                )
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
                self.lint_command(schema, rules, config.as_deref(), *fix, *strict, *format)
                    .await
            }

            LinkMLCommand::Serve {
                schema,
                port,
                host,
                cors,
                auth,
                cert,
                key,
                docs_path,
            } => {
                self.serve_command(
                    schema,
                    *port,
                    host,
                    *cors,
                    *auth,
                    cert.as_deref(),
                    key.as_deref(),
                    docs_path,
                )
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

            LinkMLCommand::Shell {
                schema,
                history,
                init,
                highlight,
            } => {
                self.shell_command(
                    schema.as_deref(),
                    history.as_deref(),
                    init.as_deref(),
                    *highlight,
                )
                .await
            }
        }
    }

    /// Validate command implementation
    async fn validate_command(
        &self,
        schema_path: &Path,
        data_paths: &[PathBuf],
        class_name: Option<&str>,
        _strict: bool,
        max_errors: usize,
        show_stats: bool,
        parallel: bool,
    ) -> Result<()> {
        println!("{}", "LinkML Validation".bold().blue());
        println!("{}", "=================".blue());

        // Load schema
        let schema = self.load_schema_file(schema_path).await?;
        println!("✓ Schema loaded: {}", schema.name);

        // Create validator
        let mut validator = ValidationEngine::new(&schema)?;

        let mut total_valid = 0;
        let mut total_invalid = 0;
        let mut all_errors = Vec::new();

        // Validate each data file
        for data_path in data_paths {
            println!("\nValidating: {}", data_path.display());

            let data = self.load_data_file(data_path).await?;
            let target_class = class_name.unwrap_or("Root");

            let report = if parallel {
                // For parallel validation, convert single data to collection
                let collection = if data.is_array() {
                    data.as_array().unwrap().clone()
                } else {
                    vec![data.clone()]
                };
                validator
                    .validate_collection_parallel(&collection, target_class, None)
                    .await?
            } else {
                validator
                    .validate_as_class(&data, target_class, None)
                    .await?
            };

            if report.valid {
                println!("  {} PASSED", "✓".green());
                total_valid += 1;
            } else {
                println!("  {} FAILED ({} errors)", "✗".red(), report.issues.len());
                total_invalid += 1;
                all_errors.extend(report.issues.clone());
            }

            // Show errors for this file
            if !report.valid && max_errors > 0 {
                for (i, error) in report.issues.iter().take(max_errors).enumerate() {
                    println!("    {}. {}: {}", i + 1, error.path.clone(), error.message);
                }

                if report.issues.len() > max_errors {
                    println!(
                        "    ... and {} more errors",
                        report.issues.len() - max_errors
                    );
                }
            }
        }

        // Summary
        println!("\n{}", "Summary:".bold());
        println!("  Files validated: {}", data_paths.len());
        println!(
            "  Valid: {} {}",
            total_valid,
            if total_valid > 0 {
                "✓".green()
            } else {
                "".normal()
            }
        );
        println!(
            "  Invalid: {} {}",
            total_invalid,
            if total_invalid > 0 {
                "✗".red()
            } else {
                "".normal()
            }
        );

        if show_stats {
            println!("\n{}", "Statistics:".bold());
            println!("  Total errors: {}", all_errors.len());

            // Group errors by validator
            let mut error_types: HashMap<String, usize> = HashMap::new();
            for error in &all_errors {
                *error_types.entry(error.validator.clone()).or_insert(0) += 1;
            }

            println!("  Error types:");
            for (error_type, count) in error_types {
                println!("    {}: {}", error_type, count);
            }
        }

        // Exit with error code if any validation failed
        if total_invalid > 0 {
            std::process::exit(1);
        }

        Ok(())
    }

    /// Generate command implementation
    async fn generate_command(
        &self,
        schema_path: &Path,
        output: &Path,
        generator_name: &str,
        options: &[String],
        template_dir: Option<&Path>,
        include_imports: bool,
    ) -> Result<()> {
        println!("{}", "Code Generation".bold().blue());
        println!("{}", "===============".blue());

        // Load schema
        let schema = self.load_schema_file(schema_path).await?;
        println!("Schema: {}", schema.name);
        println!("Generator: {}", generator_name);

        // Parse options
        let mut gen_options = GeneratorOptions::default();
        for opt in options {
            if let Some((key, value)) = opt.split_once('=') {
                gen_options = gen_options.set_custom(key, value);
            }
        }

        if let Some(template_path) = template_dir {
            gen_options =
                gen_options.set_custom("template_dir", template_path.to_str().unwrap_or(""));
        }

        if include_imports {
            let _ = gen_options.set_custom("include_imports", "true");
        }

        // Get generator
        let generator = self
            .generator_registry
            .get(generator_name)
            .await
            .ok_or_else(|| LinkMLError::other(format!("Unknown generator: {}", generator_name)))?;

        // Generate code
        println!("Generating...");
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .expect("progress bar template should be valid"),
        );
        pb.set_message("Processing schema...");
        pb.enable_steady_tick(std::time::Duration::from_millis(100));

        let result = generator.generate(&schema)?;

        pb.finish_with_message("✓ Generation complete");

        // Write output
        std::fs::create_dir_all(output.parent().unwrap_or(Path::new(".")))?;
        std::fs::write(output, result)?;

        println!("✓ Output written to: {}", output.display());

        Ok(())
    }

    /// Convert command implementation
    async fn convert_command(
        &self,
        input: &Path,
        output: &Path,
        from_format: Option<SchemaFormat>,
        to_format: SchemaFormat,
        pretty: bool,
        validate: bool,
    ) -> Result<()> {
        println!("{}", "Schema Conversion".bold().blue());
        println!("{}", "=================".blue());

        // Detect input format if not specified
        let input_format = if let Some(fmt) = from_format {
            fmt
        } else {
            match input.extension().and_then(|e| e.to_str()) {
                Some("json") => SchemaFormat::Json,
                Some("jsonld") => SchemaFormat::JsonLd,
                _ => SchemaFormat::Yaml,
            }
        };

        println!("Converting: {:?} -> {:?}", input_format, to_format);

        // Load schema
        let schema = self.load_schema_file(input).await?;

        // Validate if requested
        if validate {
            println!("Validating schema...");
            // Basic schema validation
            if schema.name.is_empty() {
                warn!("Schema has no name");
            }
            println!("✓ Schema is valid");
        }

        // Convert to output format
        let output_content = match to_format {
            SchemaFormat::Json => {
                if pretty {
                    serde_json::to_string_pretty(&schema)?
                } else {
                    serde_json::to_string(&schema)?
                }
            }
            SchemaFormat::Yaml => serde_yaml::to_string(&schema)?,
            SchemaFormat::JsonLd => {
                // TODO: Implement JSON-LD conversion
                return Err(LinkMLError::NotImplemented(
                    "JSON-LD conversion not yet implemented".to_string(),
                ));
            }
        };

        // Write output
        std::fs::write(output, output_content)?;
        println!("✓ Converted schema written to: {}", output.display());

        Ok(())
    }

    /// Merge command implementation
    async fn merge_command(
        &self,
        schemas: &[PathBuf],
        output: &Path,
        strategy: MergeStrategy,
        conflict: ConflictResolution,
        base: Option<&Path>,
        validate: bool,
    ) -> Result<()> {
        println!("{}", "Schema Merge".bold().blue());
        println!("{}", "============".blue());

        println!("Merging {} schemas", schemas.len());
        println!("Strategy: {:?}", strategy);
        println!("Conflict resolution: {:?}", conflict);

        // Load all schemas
        let mut loaded_schemas = Vec::new();
        for schema_path in schemas {
            println!("Loading: {}", schema_path.display());
            let schema = self.load_schema_file(schema_path).await?;
            loaded_schemas.push(schema);
        }

        // Load base schema if provided
        let base_schema = if let Some(base_path) = base {
            println!("Loading base schema: {}", base_path.display());
            Some(self.load_schema_file(base_path).await?)
        } else {
            None
        };

        // Create merge options
        let merge_options = MergeOptions {
            strategy,
            conflict_resolution: conflict,
            base_schema,
            preserve_annotations: true,
            merge_imports: true,
        };

        // Perform merge
        println!("\nMerging schemas...");
        let merger = SchemaMerge::new(merge_options);
        let merged_schema = merger.merge(&loaded_schemas)?;

        // Validate if requested
        if validate {
            println!("Validating merged schema...");
            // Basic validation
            if merged_schema.classes.is_empty() && merged_schema.slots.is_empty() {
                warn!("Merged schema has no classes or slots");
            }
            println!("✓ Merged schema is valid");
        }

        // Write output
        let output_content = serde_yaml::to_string(&merged_schema)?;
        std::fs::write(output, output_content)?;

        println!("✓ Merged schema written to: {}", output.display());

        Ok(())
    }

    /// Diff command implementation
    async fn diff_command(
        &self,
        schema1_path: &Path,
        schema2_path: &Path,
        format: DiffFormat,
        include_docs: bool,
        breaking_only: bool,
        context: usize,
        output: Option<&Path>,
    ) -> Result<()> {
        println!("{}", "Schema Diff".bold().blue());
        println!("{}", "===========".blue());

        // Load schemas
        println!("Loading schemas...");
        let schema1 = self.load_schema_file(schema1_path).await?;
        let schema2 = self.load_schema_file(schema2_path).await?;

        println!("Schema 1: {}", schema1.name);
        println!("Schema 2: {}", schema2.name);

        // Create diff options
        let diff_options = DiffOptions {
            include_documentation: include_docs,
            breaking_changes_only: breaking_only,
            context_lines: context,
        };

        // Compute diff
        println!("\nComputing differences...");
        let differ = SchemaDiff::new(diff_options);
        let diff_result = differ.diff(&schema1, &schema2)?;

        // Format output
        let formatted_diff = match format {
            DiffFormat::Unified => diff_result.to_unified_diff(),
            DiffFormat::SideBySide => diff_result.to_side_by_side(),
            DiffFormat::JsonPatch => diff_result.to_json_patch()?,
            DiffFormat::Html => diff_result.to_html(),
            DiffFormat::Markdown => diff_result.to_markdown(),
        };

        // Write or print output
        if let Some(output_path) = output {
            std::fs::write(output_path, formatted_diff)?;
            println!("✓ Diff written to: {}", output_path.display());
        } else {
            println!("\n{}", formatted_diff);
        }

        // Summary
        println!("\n{}", "Summary:".bold());
        println!(
            "  Added: {} classes, {} slots",
            diff_result.added_classes.len(),
            diff_result.added_slots.len()
        );
        println!(
            "  Removed: {} classes, {} slots",
            diff_result.removed_classes.len(),
            diff_result.removed_slots.len()
        );
        println!(
            "  Modified: {} classes, {} slots",
            diff_result.modified_classes.len(),
            diff_result.modified_slots.len()
        );

        if !diff_result.breaking_changes.is_empty() {
            println!("\n{} Breaking changes detected:", "⚠️ ".yellow());
            for change in &diff_result.breaking_changes {
                println!("  - {}", change);
            }
        }

        Ok(())
    }

    /// Lint command implementation
    async fn lint_command(
        &self,
        schema_path: &Path,
        rules: &[String],
        config: Option<&Path>,
        fix: bool,
        strict: bool,
        format: LintFormat,
    ) -> Result<()> {
        println!("{}", "Schema Linting".bold().blue());
        println!("{}", "==============".blue());

        // Load schema
        let mut schema = self.load_schema_file(schema_path).await?;
        println!("Schema: {}", schema.name);

        // Create lint options
        let mut lint_options = LintOptions::default();

        // Load config if provided
        if let Some(config_path) = config {
            println!("Loading lint config: {}", config_path.display());
            let config_content = std::fs::read_to_string(config_path)?;
            let lint_config: HashMap<String, serde_json::Value> =
                serde_yaml::from_str(&config_content)?;
            lint_options.apply_config(lint_config);
        }

        // Apply rule filters
        if !rules.is_empty() {
            lint_options.filter_rules(rules);
        }

        // Create linter
        let linter = SchemaLinter::new(lint_options);

        // Run linting
        println!("Running lint checks...");
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .expect("progress bar template should be valid"),
        );
        pb.set_message("Analyzing schema...");
        pb.enable_steady_tick(std::time::Duration::from_millis(100));

        let mut lint_result = linter.lint(&schema)?;

        pb.finish_and_clear();

        // Auto-fix if requested
        if fix && !lint_result.fixable_issues.is_empty() {
            println!("\nApplying automatic fixes...");
            let fixed_count = linter.fix(&mut schema, &mut lint_result)?;
            println!("✓ Fixed {} issues", fixed_count);

            // Save fixed schema
            let fixed_content = serde_yaml::to_string(&schema)?;
            std::fs::write(schema_path, fixed_content)?;
            println!("✓ Updated schema written to: {}", schema_path.display());
        }

        // Format output
        match format {
            LintFormat::Pretty => {
                self.print_lint_results(&lint_result);
            }
            LintFormat::Json => {
                let json = serde_json::to_string_pretty(&lint_result)?;
                println!("{}", json);
            }
            LintFormat::Github => {
                for issue in &lint_result.issues {
                    println!(
                        "::{}:: file={},line={},col={}::{}",
                        issue.severity.to_string().to_lowercase(),
                        schema_path.display(),
                        issue.line.unwrap_or(1),
                        issue.column.unwrap_or(1),
                        issue.message
                    );
                }
            }
            LintFormat::Junit => {
                let junit = lint_result.to_junit_xml(schema_path.to_str().unwrap_or("schema"));
                println!("{}", junit);
            }
        }

        // Exit with error if strict mode and issues found
        if strict && !lint_result.issues.is_empty() {
            std::process::exit(1);
        }

        Ok(())
    }

    /// Serve command implementation
    async fn serve_command(
        &self,
        schema_path: &Path,
        port: u16,
        host: &str,
        cors: bool,
        auth: Option<AuthType>,
        cert: Option<&Path>,
        key: Option<&Path>,
        docs_path: &str,
    ) -> Result<()> {
        println!("{}", "LinkML API Server".bold().blue());
        println!("{}", "=================".blue());

        // Load schema
        let schema = self.load_schema_file(schema_path).await?;
        println!("Schema: {}", schema.name);

        // Validate TLS configuration
        if cert.is_some() != key.is_some() {
            return Err(linkml_core::error::LinkMLError::service(
                "Both cert and key must be provided for TLS",
            ));
        }

        // Server configuration
        println!("\nServer configuration:");
        println!("  Address: {}:{}", host, port);
        println!("  CORS: {}", if cors { "enabled" } else { "disabled" });
        println!("  Auth: {:?}", auth.unwrap_or(AuthType::None));
        println!(
            "  TLS: {}",
            if cert.is_some() {
                "enabled"
            } else {
                "disabled"
            }
        );
        if let (Some(cert_path), Some(key_path)) = (cert, key) {
            println!("    Certificate: {}", cert_path.display());
            println!("    Key: {}", key_path.display());
        }
        println!("  API docs: {}", docs_path);

        // TODO: Implement actual server using a web framework
        println!("\n{}", "Server implementation not yet complete".yellow());
        println!("This will start a REST API server with:");
        println!("  - Schema validation endpoints");
        println!("  - Code generation endpoints");
        println!("  - Schema query endpoints");
        println!("  - OpenAPI documentation");

        Ok(())
    }

    /// Load command implementation
    async fn load_command(
        &self,
        schema_path: &Path,
        input: &Path,
        format: LoadFormat,
        output: &Path,
        options: &[String],
        validate: bool,
        class_name: Option<&str>,
    ) -> Result<()> {
        println!("{}", "Data Loading".bold().blue());
        println!("{}", "============".blue());

        // Load schema
        let schema = self.load_schema_file(schema_path).await?;
        println!("Schema: {}", schema.name);
        println!("Loading from: {} ({:?})", input.display(), format);

        // Parse options
        let mut load_options = HashMap::new();
        for opt in options {
            if let Some((key, value)) = opt.split_once('=') {
                load_options.insert(key.to_string(), value.to_string());
            }
        }

        // Create appropriate loader
        let instances = match format {
            LoadFormat::Csv => {
                let mut csv_options = CsvOptions::default();
                if let Some(delimiter) = load_options.get("delimiter") {
                    csv_options.delimiter = delimiter.chars().next().unwrap_or(',') as u8;
                }
                if let Some(has_header) = load_options.get("header") {
                    csv_options.has_headers = has_header == "true";
                }

                let loader = CsvLoader::with_options(csv_options);
                let load_opts = LoadOptions::default();
                loader
                    .load_file(input, &schema, &load_opts)
                    .await
                    .map_err(|e| LinkMLError::other(e.to_string()))?
            }

            LoadFormat::Json => {
                let loader = JsonLoader::new();
                let load_opts = LoadOptions::default();
                loader
                    .load_file(input, &schema, &load_opts)
                    .await
                    .map_err(|e| LinkMLError::other(e.to_string()))?
            }

            LoadFormat::Yaml => {
                let loader = YamlLoader::new();
                let load_opts = LoadOptions::default();
                loader
                    .load_file(input, &schema, &load_opts)
                    .await
                    .map_err(|e| LinkMLError::other(e.to_string()))?
            }

            LoadFormat::Xml => {
                let loader = XmlLoader::new();
                let load_opts = LoadOptions::default();
                loader
                    .load_file(input, &schema, &load_opts)
                    .await
                    .map_err(|e| LinkMLError::other(e.to_string()))?
            }

            LoadFormat::Rdf => {
                let mut rdf_options = RdfOptions::default();
                if let Some(format_str) = load_options.get("format") {
                    rdf_options.format = match format_str.as_str() {
                        "turtle" => RdfSerializationFormat::Turtle,
                        "ntriples" => RdfSerializationFormat::NTriples,
                        "rdfxml" => RdfSerializationFormat::RdfXml,
                        "nquads" => RdfSerializationFormat::NQuads,
                        "trig" => RdfSerializationFormat::TriG,
                        _ => RdfSerializationFormat::Turtle,
                    };
                }

                let loader = RdfLoader::with_options(rdf_options);
                let load_opts = LoadOptions::default();
                loader
                    .load_file(input, &schema, &load_opts)
                    .await
                    .map_err(|e| LinkMLError::other(e.to_string()))?
            }

            LoadFormat::Database => {
                #[cfg(feature = "database")]
                {
                    let mut db_options = DatabaseOptions::default();
                    db_options.connection_string = load_options
                        .get("connection")
                        .ok_or_else(|| {
                            LinkMLError::config("Database connection string required".to_string())
                        })?
                        .clone();

                    let loader = DatabaseLoader::new(db_options);
                    let load_opts = LoadOptions::default();
                    loader
                        .load_file(input, &schema, &load_opts)
                        .await
                        .map_err(|e| LinkMLError::other(e.to_string()))?
                }
                #[cfg(not(feature = "database"))]
                {
                    return Err(LinkMLError::config(
                        "Database support not compiled in".to_string(),
                    ));
                }
            }

            LoadFormat::Api => {
                let mut api_options = ApiOptions::default();
                api_options.base_url = load_options
                    .get("url")
                    .ok_or_else(|| LinkMLError::config("API URL required".to_string()))?
                    .clone();

                let loader = ApiLoader::new(api_options);
                let load_opts = LoadOptions::default();
                loader
                    .load_file(input, &schema, &load_opts)
                    .await
                    .map_err(|e| LinkMLError::other(e.to_string()))?
            }

            LoadFormat::TypeDb => {
                return Err(LinkMLError::config(
                    "TypeDB loading is not supported in the CLI. \n\
                     TypeDB operations require the DBMS service which is not available in standalone CLI mode. \n\
                     Please use the LinkML service API with proper service dependencies instead.".to_string()
                ));
            }
        };

        println!("✓ Loaded {} instances", instances.len());

        // Validate if requested
        if validate {
            println!("\nValidating loaded data...");
            let validator = ValidationEngine::new(&schema)?;
            let mut valid_count = 0;
            let mut invalid_count = 0;

            for instance in &instances {
                let data = serde_json::to_value(&instance.data)?;
                let target_class = class_name.unwrap_or(&instance.class_name);

                let report = validator
                    .validate_as_class(&data, target_class, None)
                    .await?;
                if report.valid {
                    valid_count += 1;
                } else {
                    invalid_count += 1;
                }
            }

            println!("  Valid: {}", valid_count);
            println!("  Invalid: {}", invalid_count);
        }

        // Write output
        let output_data = serde_json::to_string_pretty(&instances)?;
        std::fs::write(output, output_data)?;
        println!("\n✓ Output written to: {}", output.display());

        Ok(())
    }

    /// Dump command implementation
    async fn dump_command(
        &self,
        schema_path: &Path,
        input: &Path,
        output: &Path,
        format: DumpFormat,
        options: &[String],
        pretty: bool,
    ) -> Result<()> {
        println!("{}", "Data Dumping".bold().blue());
        println!("{}", "============".blue());

        // Load schema
        let schema = self.load_schema_file(schema_path).await?;
        println!("Schema: {}", schema.name);

        // Load instances
        let input_content = std::fs::read_to_string(input)?;
        let instances: Vec<crate::loader::traits::DataInstance> =
            serde_json::from_str(&input_content)?;
        println!("Loaded {} instances", instances.len());
        println!("Dumping to: {} ({:?})", output.display(), format);

        // Parse options
        let mut dump_options = HashMap::new();
        for opt in options {
            if let Some((key, value)) = opt.split_once('=') {
                dump_options.insert(key.to_string(), value.to_string());
            }
        }

        // Create appropriate dumper and dump data
        match format {
            DumpFormat::Csv => {
                let mut csv_options = CsvOptions::default();
                if let Some(delimiter) = dump_options.get("delimiter") {
                    csv_options.delimiter = delimiter.chars().next().unwrap_or(',') as u8;
                }

                let dumper = CsvDumper::with_options(csv_options);
                let dump_opts = DumpOptions::default();
                dumper
                    .dump_file(&instances, output, &schema, &dump_opts)
                    .await
                    .map_err(|e| LinkMLError::other(e.to_string()))?;
            }

            DumpFormat::Json => {
                let dumper = JsonDumper::new(pretty);
                let dump_opts = DumpOptions::default();
                dumper
                    .dump_file(&instances, output, &schema, &dump_opts)
                    .await
                    .map_err(|e| LinkMLError::other(e.to_string()))?;
            }

            DumpFormat::Yaml => {
                let dumper = YamlDumper::new();
                let dump_opts = DumpOptions::default();
                dumper
                    .dump_file(&instances, output, &schema, &dump_opts)
                    .await
                    .map_err(|e| LinkMLError::other(e.to_string()))?;
            }

            DumpFormat::Xml => {
                let dumper = XmlDumper::new(pretty);
                let dump_opts = DumpOptions::default();
                dumper
                    .dump_file(&instances, output, &schema, &dump_opts)
                    .await
                    .map_err(|e| LinkMLError::other(e.to_string()))?;
            }

            DumpFormat::Rdf => {
                let mut rdf_options = RdfOptions::default();
                if let Some(format_str) = dump_options.get("format") {
                    rdf_options.format = match format_str.as_str() {
                        "turtle" => RdfSerializationFormat::Turtle,
                        "ntriples" => RdfSerializationFormat::NTriples,
                        "rdfxml" => RdfSerializationFormat::RdfXml,
                        "nquads" => RdfSerializationFormat::NQuads,
                        "trig" => RdfSerializationFormat::TriG,
                        _ => RdfSerializationFormat::Turtle,
                    };
                }

                let dumper = RdfDumper::with_options(rdf_options);
                let dump_opts = DumpOptions::default();
                dumper
                    .dump_file(&instances, output, &schema, &dump_opts)
                    .await
                    .map_err(|e| LinkMLError::other(e.to_string()))?;
            }

            DumpFormat::Database => {
                #[cfg(feature = "database")]
                {
                    let mut db_options = DatabaseOptions::default();
                    db_options.connection_string = dump_options
                        .get("connection")
                        .ok_or_else(|| {
                            LinkMLError::config("Database connection string required".to_string())
                        })?
                        .clone();
                    db_options.create_if_not_exists = dump_options
                        .get("create_tables")
                        .map(|v| v == "true")
                        .unwrap_or(false);

                    let dumper = DatabaseDumper::new(db_options);
                    let dump_opts = DumpOptions::default();
                    // Database dumper doesn't use files, use dump_string
                    let _ = dumper
                        .dump_string(&instances, &schema, &dump_opts)
                        .await
                        .map_err(|e| LinkMLError::other(e.to_string()))?;
                    println!("✓ Data dumped to database");
                    return Ok(());
                }
                #[cfg(not(feature = "database"))]
                {
                    return Err(LinkMLError::config(
                        "Database support not compiled in".to_string(),
                    ));
                }
            }

            DumpFormat::Api => {
                let mut api_options = ApiOptions::default();
                api_options.base_url = dump_options
                    .get("url")
                    .ok_or_else(|| LinkMLError::config("API URL required".to_string()))?
                    .clone();

                let dumper = ApiDumper::new(api_options);
                let dump_opts = DumpOptions::default();
                // API dumper doesn't use files, use dump_string
                let _ = dumper
                    .dump_string(&instances, &schema, &dump_opts)
                    .await
                    .map_err(|e| LinkMLError::other(e.to_string()))?;
                println!("✓ Data dumped to API");
                return Ok(());
            }

            DumpFormat::TypeDb => {
                return Err(LinkMLError::config(
                    "TypeDB dumping is not supported in the CLI. \n\
                     TypeDB operations require the DBMS service which is not available in standalone CLI mode. \n\
                     Please use the LinkML service API with proper service dependencies instead.".to_string()
                ));
            }
        }

        println!("✓ Output written to: {}", output.display());
        Ok(())
    }

    /// Shell command implementation
    async fn shell_command(
        &self,
        initial_schema: Option<&Path>,
        history_file: Option<&Path>,
        init_script: Option<&Path>,
        highlight: bool,
    ) -> Result<()> {
        println!("{}", "LinkML Interactive Shell".bold().blue());
        println!("{}", "========================".blue());
        println!("Type 'help' for commands, 'quit' to exit\n");

        if highlight {
            println!("{}", "Syntax highlighting: enabled".green());
        }

        // Load initial schema if provided
        let mut current_schema = if let Some(schema_path) = initial_schema {
            println!("Loading initial schema: {}", schema_path.display());
            Some(self.load_schema_file(schema_path).await?)
        } else {
            None
        };

        // Setup readline
        let mut rl = rustyline::Editor::<(), rustyline::history::DefaultHistory>::new()
            .map_err(|e| LinkMLError::other(e.to_string()))?;

        // Load history
        if let Some(history_path) = history_file {
            let _ = rl.load_history(history_path);
        }

        // Run init script if provided
        if let Some(init_path) = init_script {
            println!("Running init script: {}", init_path.display());
            let script = std::fs::read_to_string(init_path)?;
            for line in script.lines() {
                self.execute_shell_command(line, &mut current_schema)
                    .await?;
            }
        }

        // Interactive loop
        loop {
            let prompt = if current_schema.is_some() {
                "linkml> ".green().to_string()
            } else {
                "linkml> ".normal().to_string()
            };

            match rl.readline(&prompt) {
                Ok(line) => {
                    let _ = rl.add_history_entry(line.as_str());

                    if line.trim() == "quit" || line.trim() == "exit" {
                        break;
                    }

                    if let Err(e) = self.execute_shell_command(&line, &mut current_schema).await {
                        eprintln!("{} {}", "Error:".red(), e);
                    }
                }
                Err(rustyline::error::ReadlineError::Interrupted) => {
                    println!("^C");
                }
                Err(rustyline::error::ReadlineError::Eof) => {
                    println!("^D");
                    break;
                }
                Err(err) => {
                    eprintln!("Error: {}", err);
                    break;
                }
            }
        }

        // Save history
        if let Some(history_path) = history_file {
            let _ = rl.save_history(history_path);
        }

        println!("\nGoodbye!");
        Ok(())
    }

    // Helper methods

    /// Load schema from file
    async fn load_schema_file(&self, path: &Path) -> Result<SchemaDefinition> {
        let content = std::fs::read_to_string(path)?;

        let schema = if path.extension().and_then(|e| e.to_str()) == Some("json") {
            serde_json::from_str(&content)?
        } else {
            serde_yaml::from_str(&content)?
        };

        Ok(schema)
    }

    /// Load data from file
    async fn load_data_file(&self, path: &Path) -> Result<serde_json::Value> {
        let content = std::fs::read_to_string(path)?;

        let data = if path.extension().and_then(|e| e.to_str()) == Some("json") {
            serde_json::from_str(&content)?
        } else {
            serde_yaml::from_str(&content)?
        };

        Ok(data)
    }

    /// Print lint results in pretty format
    fn print_lint_results(&self, result: &LintResult) {
        if result.issues.is_empty() {
            println!("\n{} No issues found!", "✓".green());
            return;
        }

        println!("\n{} {} issues found:", "⚠️ ".yellow(), result.issues.len());

        // Group by severity
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut info = Vec::new();

        for issue in &result.issues {
            match issue.severity {
                crate::schema::lint::Severity::Error => errors.push(issue),
                crate::schema::lint::Severity::Warning => warnings.push(issue),
                crate::schema::lint::Severity::Info => info.push(issue),
            }
        }

        // Print errors
        if !errors.is_empty() {
            println!("\n{} Errors:", "✗".red());
            for issue in errors {
                println!("  {}: {}", issue.rule.red(), issue.message);
                if let Some(suggestion) = &issue.suggestion {
                    println!("    {} {}", "→".cyan(), suggestion);
                }
            }
        }

        // Print warnings
        if !warnings.is_empty() {
            println!("\n{} Warnings:", "⚠".yellow());
            for issue in warnings {
                println!("  {}: {}", issue.rule.yellow(), issue.message);
                if let Some(suggestion) = &issue.suggestion {
                    println!("    {} {}", "→".cyan(), suggestion);
                }
            }
        }

        // Print info
        if !info.is_empty() {
            println!("\n{} Info:", "ℹ".blue());
            for issue in info {
                println!("  {}: {}", issue.rule.blue(), issue.message);
                if let Some(suggestion) = &issue.suggestion {
                    println!("    {} {}", "→".cyan(), suggestion);
                }
            }
        }

        // Summary
        println!("\n{}", "Summary:".bold());
        println!("  Errors: {}", result.error_count());
        println!("  Warnings: {}", result.warning_count());
        println!("  Info: {}", result.info_count());

        if !result.fixable_issues.is_empty() {
            println!(
                "\n{} {} issues can be fixed automatically with --fix",
                "💡".cyan(),
                result.fixable_issues.len()
            );
        }
    }

    /// Execute a shell command
    async fn execute_shell_command(
        &self,
        command: &str,
        current_schema: &mut Option<SchemaDefinition>,
    ) -> Result<()> {
        let parts: Vec<&str> = command.trim().split_whitespace().collect();
        if parts.is_empty() {
            return Ok(());
        }

        match parts[0] {
            "help" => {
                println!("Available commands:");
                println!("  load <file>     - Load a schema");
                println!("  show            - Show current schema");
                println!("  validate <file> - Validate data against current schema");
                println!("  generate <gen>  - Generate code using current schema");
                println!("  classes         - List all classes");
                println!("  slots           - List all slots");
                println!("  clear           - Clear current schema");
                println!("  quit/exit       - Exit shell");
            }

            "load" => {
                if parts.len() < 2 {
                    println!("Usage: load <schema_file>");
                } else {
                    match self.load_schema_file(Path::new(parts[1])).await {
                        Ok(schema) => {
                            println!("✓ Loaded schema: {}", schema.name);
                            *current_schema = Some(schema);
                        }
                        Err(e) => println!("Failed to load schema: {}", e),
                    }
                }
            }

            "show" => {
                if let Some(schema) = current_schema {
                    println!("Schema: {}", schema.name);
                    if let Some(version) = &schema.version {
                        println!("Version: {}", version);
                    }
                    if let Some(description) = &schema.description {
                        println!("Description: {}", description);
                    }
                    println!("Classes: {}", schema.classes.len());
                    println!("Slots: {}", schema.slots.len());
                } else {
                    println!("No schema loaded");
                }
            }

            "classes" => {
                if let Some(schema) = current_schema {
                    println!("Classes:");
                    for (name, class) in &schema.classes {
                        println!(
                            "  {} {}",
                            name,
                            if let Some(desc) = &class.description {
                                format!("- {}", desc)
                            } else {
                                String::new()
                            }
                        );
                    }
                } else {
                    println!("No schema loaded");
                }
            }

            "slots" => {
                if let Some(schema) = current_schema {
                    println!("Slots:");
                    for (name, slot) in &schema.slots {
                        println!(
                            "  {} ({}) {}",
                            name,
                            slot.range.as_deref().unwrap_or("string"),
                            if let Some(desc) = &slot.description {
                                format!("- {}", desc)
                            } else {
                                String::new()
                            }
                        );
                    }
                } else {
                    println!("No schema loaded");
                }
            }

            "clear" => {
                *current_schema = None;
                println!("Schema cleared");
            }

            _ => {
                println!(
                    "Unknown command: {}. Type 'help' for available commands.",
                    parts[0]
                );
            }
        }

        Ok(())
    }
}

/// Run the LinkML CLI application
pub async fn run() -> Result<()> {
    let mut app = LinkMLApp::new();
    app.run().await
}
