//! CLI type definitions and enums

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// `LinkML` command-line interface
#[derive(Parser, Debug)]
#[command(name = "linkml", version, about = "LinkML schema tools")]
pub struct LinkMLCli {
    /// Enable verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Quiet mode - suppress non-essential output
    #[arg(short, long)]
    pub quiet: bool,

    /// Output format
    #[arg(short = 'f', long, global = true, default_value = "pretty")]
    pub format: OutputFormat,

    /// Command to execute
    #[command(subcommand)]
    pub command: LinkMLCommand,
}

/// Output formats for CLI commands
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

/// `LinkML` subcommands
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
        /// Input data file (`LinkML` format)
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

    /// Interactive `LinkML` shell
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

    /// Convert Excel SchemaSheets to LinkML schema
    ///
    /// Analyzes an Excel workbook containing data and generates a LinkML schema
    /// by inferring types, constraints, and relationships from the data.
    ///
    /// # Examples
    ///
    /// ```bash
    /// # Generate schema from Excel data
    /// linkml sheets2schema data.xlsx -o schema.yaml
    ///
    /// # Specify schema ID and name
    /// linkml sheets2schema data.xlsx --schema-id my_schema --schema-name "My Schema"
    ///
    /// # Output as JSON
    /// linkml sheets2schema data.xlsx -o schema.json --format json
    /// ```
    #[command(name = "sheets2schema")]
    Sheets2Schema {
        /// Input Excel file path (.xlsx, .xls, .xlsb, .ods)
        #[arg(value_name = "EXCEL_FILE")]
        input: PathBuf,

        /// Output schema file path (defaults to <input>.yaml)
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,

        /// Schema ID (defaults to filename without extension)
        #[arg(long, value_name = "ID")]
        schema_id: Option<String>,

        /// Schema name (defaults to schema ID)
        #[arg(long, value_name = "NAME")]
        schema_name: Option<String>,

        /// Output format (yaml or json)
        #[arg(short = 'f', long, default_value = "yaml")]
        format: SchemaFormat,

        /// Show progress indicators
        #[arg(long, default_value = "true")]
        progress: bool,
    },

    /// Convert LinkML schema to Excel SchemaSheets template
    ///
    /// Generates an Excel workbook template from a LinkML schema definition.
    /// The template includes sheets for each class with appropriate columns,
    /// data validation, and formatting.
    ///
    /// # Examples
    ///
    /// ```bash
    /// # Generate Excel template from schema
    /// linkml schema2sheets schema.yaml -o template.xlsx
    ///
    /// # Include data validation and examples
    /// linkml schema2sheets schema.yaml -o template.xlsx --validation --examples
    ///
    /// # Customize formatting
    /// linkml schema2sheets schema.yaml -o template.xlsx --freeze-headers --filters
    /// ```
    #[command(name = "schema2sheets")]
    Schema2Sheets {
        /// Input schema file path (.yaml, .yml, .json)
        #[arg(value_name = "SCHEMA_FILE")]
        schema: PathBuf,

        /// Output Excel file path
        #[arg(short, long, value_name = "FILE")]
        output: PathBuf,

        /// Add data validation (dropdowns, ranges)
        #[arg(long)]
        validation: bool,

        /// Include example data rows
        #[arg(long)]
        examples: bool,

        /// Freeze header rows
        #[arg(long, default_value = "true")]
        freeze_headers: bool,

        /// Add auto-filters to columns
        #[arg(long, default_value = "true")]
        filters: bool,

        /// Show progress indicators
        #[arg(long, default_value = "true")]
        progress: bool,
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
    /// `JUnit` XML format
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
    /// `TypeDB`
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
    /// `TypeDB`
    TypeDb,
}
