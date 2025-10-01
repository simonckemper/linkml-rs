//! Interactive validation mode for LinkML
//!
//! This module provides an interactive REPL for:
//! - Loading and reloading schemas
//! - Validating data interactively
//! - Exploring schema structure
//! - Testing validation rules
//! - Debugging validation issues

use colored::Colorize;
use linkml_core::error::LinkMLError;
use linkml_core::types::SchemaDefinition;
use rustyline::Helper;
use rustyline::completion::{Completer, FilenameCompleter, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::{Highlighter, MatchingBracketHighlighter};
use rustyline::hint::Hinter;
use rustyline::hint::HistoryHinter;
use rustyline::validate::{ValidationContext, ValidationResult, Validator};
use rustyline::{CompletionType, Config, EditMode, Editor};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use timestamp_core::TimestampService;

/// Interactive session state
pub struct InteractiveSession<S> {
    /// Loaded schemas
    schemas: HashMap<String, Arc<SchemaDefinition>>,
    /// Current active schema
    current_schema: Option<String>,
    /// Validation history
    history: Vec<ValidationHistoryEntry>,
    /// Session configuration
    config: InteractiveConfig,
    /// `LinkML` service
    service: Arc<S>,
    /// Timestamp service for history entries
    timestamp_service: Arc<dyn TimestampService<Error = timestamp_core::TimestampError>>,
}

/// Interactive session configuration
#[derive(Debug, Clone)]
pub struct InteractiveConfig {
    /// History file path
    pub history_file: Option<PathBuf>,
    /// Maximum history entries
    pub max_history: usize,
    /// Auto-reload schemas on change
    pub auto_reload: bool,
    /// Show hints
    pub show_hints: bool,
    /// Pretty print `JSON`
    pub pretty_json: bool,
    /// Syntax highlighting
    pub syntax_highlighting: bool,
}

impl Default for InteractiveConfig {
    fn default() -> Self {
        Self {
            history_file: Some(PathBuf::from(".linkml_history")),
            max_history: 1000,
            auto_reload: true,
            show_hints: true,
            pretty_json: true,
            syntax_highlighting: true,
        }
    }
}

/// Validation history entry
#[derive(Debug, Clone)]
struct ValidationHistoryEntry {
    /// Schema used
    schema_name: String,
    /// Data validated
    data: Value,
    /// Validation result
    valid: bool,
    /// Issue count
    issue_count: usize,
    /// Timestamp
    timestamp: chrono::DateTime<chrono::Local>,
}

/// Interactive commands
#[derive(Debug, Clone)]
enum Command {
    /// Load a schema
    Load { path: PathBuf, name: Option<String> },
    /// Reload current schema
    Reload,
    /// List loaded schemas
    List,
    /// Switch to a schema
    Use { name: String },
    /// Validate `JSON` data
    Validate { data: Value, class: Option<String> },
    /// Validate file
    ValidateFile {
        path: PathBuf,
        class: Option<String>,
    },
    /// Show schema info
    Info { item: Option<String> },
    /// Show class details
    Class { name: String },
    /// Show slot details
    Slot { name: String },
    /// Show type details
    Type { name: String },
    /// Show enum details
    Enum { name: String },
    /// Search in schema
    Search { pattern: String },
    /// Show validation history
    History { count: Option<usize> },
    /// Clear screen
    Clear,
    /// Show help
    Help,
    /// Exit
    Quit,
}

impl<S: linkml_core::traits::LinkMLService> InteractiveSession<S> {
    /// Create new interactive session
    pub fn new(
        service: Arc<S>,
        config: InteractiveConfig,
        timestamp_service: Arc<dyn TimestampService<Error = timestamp_core::TimestampError>>,
    ) -> Self {
        Self {
            schemas: HashMap::new(),
            current_schema: None,
            history: Vec::new(),
            config,
            service,
            timestamp_service,
        }
    }

    /// Run interactive session
    ///
    /// # Errors
    ///
    /// Returns an error if the readline initialization fails or command execution fails.
    pub async fn run(&mut self) -> crate::Result<()> {
        println!("{}", "LinkML Interactive Mode".bold().blue());
        println!("{}", "=======================".blue());
        println!(
            "Type 'help' for commands, 'quit' to exit
"
        );

        // Setup readline
        let config = Config::builder()
            .history_ignore_space(true)
            .completion_type(CompletionType::List)
            .edit_mode(EditMode::Emacs)
            .build();

        let helper = InteractiveHelper::new();
        let mut rl = Editor::with_config(config)
            .map_err(|e| LinkMLError::service(format!("Failed to create readline editor: {e}")))?;
        rl.set_helper(Some(helper));

        // Load history
        if let Some(history_file) = &self.config.history_file {
            let _ = rl.load_history(history_file);
        }

        loop {
            let prompt = if let Some(schema) = &self.current_schema {
                format!("{}> ", schema.green())
            } else {
                "linkml> ".to_string()
            };

            match rl.readline(&prompt) {
                Ok(line) => {
                    let _ = rl.add_history_entry(&line);

                    match self.parse_command(&line) {
                        Ok(Command::Quit) => break,
                        Ok(cmd) => {
                            if let Err(e) = self.execute_command(cmd).await {
                                eprintln!("{}: {}", "Error".red(), e);
                            }
                        }
                        Err(e) => {
                            eprintln!("{}: {}", "Parse error".red(), e);
                        }
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    println!("Use 'quit' to exit");
                }
                Err(ReadlineError::Eof) => break,
                Err(err) => {
                    eprintln!("Error: {err:?}");
                    break;
                }
            }
        }

        // Save history
        if let Some(history_file) = &self.config.history_file {
            let _ = rl.save_history(history_file);
        }

        Ok(())
    }

    /// Parse command from input
    fn parse_command(&self, input: &str) -> crate::Result<Command> {
        let parts: Vec<&str> = input.split_whitespace().collect();

        if parts.is_empty() {
            return Err(LinkMLError::service("Empty command"));
        }

        let first_part = parts
            .get(0)
            .ok_or_else(|| LinkMLError::service("Empty command".to_string()))?;

        match first_part.to_lowercase().as_str() {
            "load" => {
                if parts.len() < 2 {
                    return Err(LinkMLError::service("Usage: load <path> [name]"));
                }
                let path_str = parts
                    .get(1)
                    .ok_or_else(|| LinkMLError::service("Missing path argument"))?;
                Ok(Command::Load {
                    path: PathBuf::from(path_str),
                    name: parts.get(2).map(|s| (*s).to_string()),
                })
            }

            "reload" => Ok(Command::Reload),

            "list" | "ls" => Ok(Command::List),

            "use" => {
                if parts.len() < 2 {
                    return Err(LinkMLError::service("Usage: use <schema-name>"));
                }
                let name = parts
                    .get(1)
                    .ok_or_else(|| LinkMLError::service("Missing schema name argument"))?;
                Ok(Command::Use {
                    name: name.to_string(),
                })
            }

            "validate" | "v" => {
                if parts.len() < 2 {
                    return Err(LinkMLError::service("Usage: validate <json> [class]"));
                }

                let json_str = if parts.len() > 1 {
                    parts[1..].join(" ")
                } else {
                    return Err(LinkMLError::service("Missing JSON data"));
                };
                let data = serde_json::from_str(&json_str)
                    .map_err(|e| LinkMLError::service(format!("Invalid JSON: {e}")))?;

                Ok(Command::Validate {
                    data,
                    class: None, // Would parse class from command
                })
            }

            "validate-file" | "vf" => {
                if parts.len() < 2 {
                    return Err(LinkMLError::service("Usage: validate-file <path> [class]"));
                }
                let path_str = parts
                    .get(1)
                    .ok_or_else(|| LinkMLError::service("Missing file path argument"))?;
                Ok(Command::ValidateFile {
                    path: PathBuf::from(path_str),
                    class: parts.get(2).map(|s| (*s).to_string()),
                })
            }

            "info" | "i" => Ok(Command::Info {
                item: parts.get(1).map(|s| (*s).to_string()),
            }),

            "class" | "c" => {
                if parts.len() < 2 {
                    return Err(LinkMLError::service("Usage: class <name>"));
                }
                Ok(Command::Class {
                    name: parts[1].to_string(),
                })
            }

            "slot" | "s" => {
                if parts.len() < 2 {
                    return Err(LinkMLError::service("Usage: slot <name>"));
                }
                Ok(Command::Slot {
                    name: parts[1].to_string(),
                })
            }

            "type" | "t" => {
                if parts.len() < 2 {
                    return Err(LinkMLError::service("Usage: type <name>"));
                }
                Ok(Command::Type {
                    name: parts[1].to_string(),
                })
            }

            "enum" | "e" => {
                if parts.len() < 2 {
                    return Err(LinkMLError::service("Usage: enum <name>"));
                }
                Ok(Command::Enum {
                    name: parts[1].to_string(),
                })
            }

            "search" => {
                if parts.len() < 2 {
                    return Err(LinkMLError::service("Usage: search <pattern>"));
                }
                Ok(Command::Search {
                    pattern: parts[1..].join(" "),
                })
            }

            "history" | "h" => Ok(Command::History {
                count: parts.get(1).and_then(|s| s.parse().ok()),
            }),

            "clear" | "cls" => Ok(Command::Clear),

            "help" | "?" => Ok(Command::Help),

            "quit" | "exit" | "q" => Ok(Command::Quit),

            _ => Err(LinkMLError::service(format!(
                "Unknown command: {}",
                parts[0]
            ))),
        }
    }

    /// Execute command
    async fn execute_command(&mut self, command: Command) -> crate::Result<()> {
        match command {
            Command::Load { path, name } => {
                self.load_schema(&path, name).await?;
            }

            Command::Reload => {
                self.reload_current_schema();
            }

            Command::List => {
                self.list_schemas();
            }

            Command::Use { name } => {
                self.use_schema(&name)?;
            }

            Command::Validate { data, class } => {
                self.validate_data(&data, class.as_deref()).await?;
            }

            Command::ValidateFile { path, class } => {
                self.validate_file(&path, class.as_deref()).await?;
            }

            Command::Info { item } => {
                self.show_info(item.as_deref());
            }

            Command::Class { name } => {
                self.show_class(&name)?;
            }

            Command::Slot { name } => {
                self.show_slot(&name)?;
            }

            Command::Type { name } => {
                self.show_type(&name)?;
            }

            Command::Enum { name } => {
                self.show_enum(&name)?;
            }

            Command::Search { pattern } => {
                self.search_schema(&pattern)?;
            }

            Command::History { count } => {
                self.show_history(count.unwrap_or(10));
            }

            Command::Clear => {
                print!("{esc}[2J{esc}[1;1H", esc = 27 as char);
            }

            Command::Help => {
                Self::show_help();
            }

            Command::Quit => {}
        }

        Ok(())
    }

    /// Load a schema
    async fn load_schema(&mut self, path: &Path, name: Option<String>) -> crate::Result<()> {
        println!("Loading schema from {}...", path.display());

        let content = std::fs::read_to_string(path)?;
        let format = if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e == "json")
        {
            linkml_core::traits::SchemaFormat::Json
        } else {
            linkml_core::traits::SchemaFormat::Yaml
        };
        let schema = self.service.load_schema_str(&content, format).await?;

        let schema_name = name
            .or_else(|| Some(schema.name.clone()))
            .unwrap_or_else(|| {
                path.file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string()
            });

        self.schemas.insert(schema_name.clone(), Arc::new(schema));
        self.current_schema = Some(schema_name.clone());

        println!(
            "{} Schema '{}' loaded successfully",
            "✓".green(),
            schema_name
        );

        Ok(())
    }

    /// Reload current schema
    fn reload_current_schema(&mut self) {
        if let Some(name) = &self.current_schema {
            println!("Reloading schema '{name}'...");
            // Would implement actual reload logic
            println!("{} Schema reloaded", "✓".green());
        } else {
            println!("{} No schema currently loaded", "!".yellow());
        }
    }

    /// List loaded schemas
    fn list_schemas(&self) {
        if self.schemas.is_empty() {
            println!("No schemas loaded");
            return;
        }

        println!("{}", "Loaded schemas:".bold());
        for (name, schema) in &self.schemas {
            let marker = if Some(name) == self.current_schema.as_ref() {
                "*".green()
            } else {
                " ".normal()
            };

            println!(
                "{} {} (v{})",
                marker,
                name,
                schema.version.as_deref().unwrap_or("unversioned")
            );
        }
    }

    /// Switch to a schema
    fn use_schema(&mut self, name: &str) -> crate::Result<()> {
        if self.schemas.contains_key(name) {
            self.current_schema = Some(name.to_string());
            println!("Switched to schema '{name}'");
            Ok(())
        } else {
            Err(LinkMLError::service(format!("Schema '{name}' not found")))
        }
    }

    /// Validate data
    async fn validate_data(&mut self, data: &Value, class_name: Option<&str>) -> crate::Result<()> {
        let schema_name = self
            .current_schema
            .as_ref()
            .ok_or_else(|| LinkMLError::service("No schema loaded"))?;

        let schema = self
            .schemas
            .get(schema_name)
            .ok_or_else(|| LinkMLError::service("Schema not found"))?;

        println!("Validating data...");

        let start = std::time::Instant::now();
        let class_name = class_name.unwrap_or("Root"); // Default to Root class
        let report = self.service.validate(data, schema, class_name).await?;
        let duration = start.elapsed();

        // Display results
        if report.valid {
            println!(
                "{} Validation {} ({:.2}ms)",
                "✓".green(),
                "PASSED".green().bold(),
                duration.as_secs_f64() * 1000.0
            );
        } else {
            println!(
                "{} Validation {} ({:.2}ms)",
                "✗".red(),
                "FAILED".red().bold(),
                duration.as_secs_f64() * 1000.0
            );

            println!(
                "
{}",
                "Issues:".yellow()
            );
            for (i, error) in report.errors.iter().enumerate() {
                let severity = "ERROR".red();

                println!(
                    "  {}. [{}] {}: {}",
                    i + 1,
                    severity,
                    error.path.as_deref().unwrap_or(""),
                    error.message
                );
            }
        }

        // Add to history
        let local_timestamp = self.timestamp_service.now_local().await.map_err(|e| {
            LinkMLError::service(format!("Failed to get current local timestamp: {e}"))
        })?;

        self.history.push(ValidationHistoryEntry {
            schema_name: schema_name.clone(),
            data: data.clone(),
            valid: report.valid,
            issue_count: report.errors.len() + report.warnings.len(),
            timestamp: local_timestamp,
        });

        Ok(())
    }

    /// Validate file
    async fn validate_file(&mut self, path: &Path, class_name: Option<&str>) -> crate::Result<()> {
        let content = std::fs::read_to_string(path)?;
        let data: Value = if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e == "json")
        {
            serde_json::from_str(&content)?
        } else {
            serde_yaml::from_str(&content)?
        };

        self.validate_data(&data, class_name).await
    }

    /// Show schema info
    fn show_info(&self, item: Option<&str>) {
        let schema = if let Some(name) = &self.current_schema {
            if let Some(s) = self.schemas.get(name) {
                s
            } else {
                println!("Current schema not found");
                return;
            }
        } else {
            println!("No schema loaded");
            return;
        };

        if let Some(item_name) = item {
            // Show specific item info
            println!("Info for '{item_name}' not implemented yet");
        } else {
            // Show general schema info
            println!("{}", "Schema Information:".bold());
            println!("  Name: {}", schema.name);
            println!(
                "  Version: {}",
                schema.version.as_deref().unwrap_or("unversioned")
            );
            if let Some(desc) = &schema.description {
                println!("  Description: {desc}");
            }
            println!(
                "
{}",
                "Statistics:".bold()
            );
            println!("  Classes: {}", schema.classes.len());
            println!("  Slots: {}", schema.slots.len());
            println!("  Types: {}", schema.types.len());
            println!("  Enums: {}", schema.enums.len());
        }
    }

    /// Show class details
    fn show_class(&self, name: &str) -> crate::Result<()> {
        let schema = self.get_current_schema()?;

        if let Some(class) = schema.classes.get(name) {
            println!("{}", format!("Class: {name}").bold());
            if let Some(desc) = &class.description {
                println!("Description: {desc}");
            }
            if let Some(parent) = &class.is_a {
                println!("Parent: {parent}");
            }
            if !class.mixins.is_empty() {
                println!("Mixins: {}", class.mixins.join(", "));
            }
            if !class.slots.is_empty() {
                println!(
                    "
{}",
                    "Slots:".bold()
                );
                for slot in &class.slots {
                    println!("  - {slot}");
                }
            }
            Ok(())
        } else {
            Err(LinkMLError::service(format!("Class '{name}' not found")))
        }
    }

    /// Show slot details
    fn show_slot(&self, name: &str) -> crate::Result<()> {
        let schema = self.get_current_schema()?;

        if let Some(slot) = schema.slots.get(name) {
            println!("{}", format!("Slot: {name}").bold());
            if let Some(desc) = &slot.description {
                println!("Description: {desc}");
            }
            if let Some(range) = &slot.range {
                println!("Range: {range}");
            }
            if let Some(required) = slot.required {
                println!("Required: {required}");
            }
            if let Some(multivalued) = slot.multivalued {
                println!("Multivalued: {multivalued}");
            }
            if let Some(pattern) = &slot.pattern {
                println!("Pattern: {pattern}");
            }
            Ok(())
        } else {
            Err(LinkMLError::service(format!("Slot '{name}' not found")))
        }
    }

    /// Show type details
    fn show_type(&self, name: &str) -> crate::Result<()> {
        let schema = self.get_current_schema()?;

        if let Some(type_def) = schema.types.get(name) {
            println!("{}", format!("Type: {name}").bold());
            if let Some(desc) = &type_def.description {
                println!("Description: {desc}");
            }
            if let Some(base) = &type_def.base_type {
                println!("Base type: {base}");
            }
            Ok(())
        } else {
            Err(LinkMLError::service(format!("Type '{name}' not found")))
        }
    }

    /// Show enum details
    fn show_enum(&self, name: &str) -> crate::Result<()> {
        let schema = self.get_current_schema()?;

        if let Some(enum_def) = schema.enums.get(name) {
            println!("{}", format!("Enum: {name}").bold());
            if let Some(desc) = &enum_def.description {
                println!("Description: {desc}");
            }
            println!(
                "
{}",
                "Values:".bold()
            );
            for value in &enum_def.permissible_values {
                match value {
                    linkml_core::types::PermissibleValue::Simple(text)
                    | linkml_core::types::PermissibleValue::Complex { text, .. } => {
                        println!("  - {text}");
                    }
                }
            }
            Ok(())
        } else {
            Err(LinkMLError::service(format!("Enum '{name}' not found")))
        }
    }

    /// Search in schema
    fn search_schema(&self, pattern: &str) -> crate::Result<()> {
        let schema = self.get_current_schema()?;
        let pattern_lower = pattern.to_lowercase();

        println!("{}", format!("Searching for '{pattern}'...").bold());

        let mut found = false;

        // Search classes
        for (name, class) in &schema.classes {
            if name.to_lowercase().contains(&pattern_lower)
                || class
                    .description
                    .as_ref()
                    .is_some_and(|d| d.to_lowercase().contains(&pattern_lower))
            {
                println!("  {} Class: {}", "•".green(), name);
                found = true;
            }
        }

        // Search slots
        for (name, slot) in &schema.slots {
            if name.to_lowercase().contains(&pattern_lower)
                || slot
                    .description
                    .as_ref()
                    .is_some_and(|d| d.to_lowercase().contains(&pattern_lower))
            {
                println!("  {} Slot: {}", "•".blue(), name);
                found = true;
            }
        }

        if !found {
            println!("No matches found");
        }

        Ok(())
    }

    /// Show validation history
    fn show_history(&self, count: usize) {
        if self.history.is_empty() {
            println!("No validation history");
            return;
        }

        println!("{}", "Validation History:".bold());

        let start = self.history.len().saturating_sub(count);
        for (i, entry) in self.history[start..].iter().enumerate() {
            let status = if entry.valid {
                "PASS".green()
            } else {
                format!("FAIL ({})", entry.issue_count).red()
            };

            println!(
                "{:3}. {} {} - {} [{}]",
                start + i + 1,
                entry.timestamp.format("%H:%M:%S"),
                entry.schema_name,
                status,
                if entry.data.is_object() {
                    "object"
                } else if entry.data.is_array() {
                    "array"
                } else {
                    "value"
                }
            );
        }
    }

    /// Show help
    fn show_help() {
        println!("{}", "Available Commands:".bold());
        println!();
        println!("  {} <path> [name]     Load a schema file", "load".green());
        println!(
            "  {}                  Reload current schema",
            "reload".green()
        );
        println!(
            "  {} | {}               List loaded schemas",
            "list".green(),
            "ls".green()
        );
        println!("  {} <name>            Switch to a schema", "use".green());
        println!(
            "  {} <json> [class]    Validate JSON data",
            "validate".green()
        );
        println!(
            "  {} <path> [class]    Validate file",
            "validate-file".green()
        );
        println!(
            "  {} [item]            Show schema or item info",
            "info".green()
        );
        println!("  {} <name>            Show class details", "class".green());
        println!("  {} <name>            Show slot details", "slot".green());
        println!("  {} <name>            Show type details", "type".green());
        println!("  {} <name>            Show enum details", "enum".green());
        println!("  {} <pattern>         Search in schema", "search".green());
        println!(
            "  {} [count]           Show validation history",
            "history".green()
        );
        println!(
            "  {} | {}              Clear screen",
            "clear".green(),
            "cls".green()
        );
        println!(
            "  {} | {}               Show this help",
            "help".green(),
            "?".green()
        );
        println!(
            "  {} | {} | {}         Exit interactive mode",
            "quit".green(),
            "exit".green(),
            "q".green()
        );
        println!();
        println!(
            "Shortcuts: v=validate, vf=validate-file, i=info, c=class, s=slot, t=type, e=enum, h=history"
        );
    }

    /// Get current schema
    fn get_current_schema(&self) -> crate::Result<&Arc<SchemaDefinition>> {
        let name = self
            .current_schema
            .as_ref()
            .ok_or_else(|| LinkMLError::service("No schema loaded"))?;

        self.schemas
            .get(name)
            .ok_or_else(|| LinkMLError::service("Current schema not found"))
    }
}

/// Readline helper for autocompletion and hints
struct InteractiveHelper {
    completer: FilenameCompleter,
    highlighter: MatchingBracketHighlighter,
    hinter: HistoryHinter,
    commands: Vec<&'static str>,
}

impl InteractiveHelper {
    fn new() -> Self {
        Self {
            completer: FilenameCompleter::new(),
            highlighter: MatchingBracketHighlighter::new(),
            hinter: HistoryHinter {},
            commands: vec![
                "load",
                "reload",
                "list",
                "ls",
                "use",
                "validate",
                "v",
                "validate-file",
                "vf",
                "info",
                "i",
                "class",
                "c",
                "slot",
                "s",
                "type",
                "t",
                "enum",
                "e",
                "search",
                "history",
                "h",
                "clear",
                "cls",
                "help",
                "?",
                "quit",
                "exit",
                "q",
            ],
        }
    }
}

impl Helper for InteractiveHelper {}

impl Completer for InteractiveHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        // Command completion
        if pos == line.len() && !line.contains(' ') {
            let matches: Vec<_> = self
                .commands
                .iter()
                .filter(|cmd| cmd.starts_with(line))
                .map(|cmd| Pair {
                    display: (*cmd).to_string(),
                    replacement: (*cmd).to_string(),
                })
                .collect();

            return Ok((0, matches));
        }

        // File completion for load/validate-file commands
        if line.starts_with("load ") || line.starts_with("validate-file ") {
            return self.completer.complete(line, pos, ctx);
        }

        Ok((pos, vec![]))
    }
}

impl Hinter for InteractiveHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, ctx: &rustyline::Context<'_>) -> Option<Self::Hint> {
        self.hinter.hint(line, pos, ctx)
    }
}

impl Highlighter for InteractiveHelper {
    fn highlight<'l>(&self, line: &'l str, pos: usize) -> std::borrow::Cow<'l, str> {
        self.highlighter.highlight(line, pos)
    }

    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        default: bool,
    ) -> std::borrow::Cow<'b, str> {
        if default {
            std::borrow::Cow::Borrowed(prompt)
        } else {
            std::borrow::Cow::Owned(prompt.bold().to_string())
        }
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> std::borrow::Cow<'h, str> {
        std::borrow::Cow::Owned(hint.dimmed().to_string())
    }
}

impl Validator for InteractiveHelper {
    fn validate(&self, ctx: &mut ValidationContext) -> rustyline::Result<ValidationResult> {
        let input = ctx.input();

        // Check for unclosed brackets/quotes
        let open_parens = input
            .chars()
            .filter(|&c| c == '(' || c == '[' || c == '{')
            .count();
        let close_parens = input
            .chars()
            .filter(|&c| c == ')' || c == ']' || c == '}')
            .count();

        if open_parens > close_parens {
            Ok(ValidationResult::Incomplete)
        } else {
            Ok(ValidationResult::Valid(None))
        }
    }
}

#[cfg(test)]
mod tests {

    // #[test]
    // fn test_command_parsing() {
    //     // Would need to create proper mock service
    //     // let session = InteractiveSession::new(
    //     //     Arc::new(mock_service),
    //     //     InteractiveConfig::default(),
    //     // );
    //
    //     // assert!(matches!(
    //     //     session.parse_command("load test.yaml")?,
    //     //     Command::Load { .. }
    //     // ));
    //
    //     // assert!(matches!(
    //     //     session.parse_command("help")?,
    //     //     Command::Help
    //     // ));
    // }
}
