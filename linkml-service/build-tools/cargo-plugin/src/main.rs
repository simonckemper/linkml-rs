//! Cargo plugin for LinkML schema validation and code generation
//!
//! This plugin provides Cargo subcommands for working with LinkML schemas
//! in Rust projects.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

mod config;
mod generator;
mod validator;

use config::LinkMLConfig;

/// Cargo LinkML plugin
#[derive(Parser)]
#[command(name = "cargo")]
#[command(bin_name = "cargo")]
enum Cargo {
    /// LinkML schema tools
    #[command(subcommand)]
    Linkml(LinkMLCommand),
}

/// LinkML subcommands
#[derive(Subcommand)]
enum LinkMLCommand {
    /// Validate LinkML schemas
    Validate {
        /// Schema directory (default: src/schemas)
        #[arg(short, long, default_value = "src/schemas")]
        schema_dir: PathBuf,
        
        /// Include patterns
        #[arg(short, long)]
        include: Vec<String>,
        
        /// Exclude patterns
        #[arg(short = 'x', long)]
        exclude: Vec<String>,
        
        /// Fail on validation errors
        #[arg(long, default_value = "true")]
        fail_on_error: bool,
        
        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },
    
    /// Generate Rust code from LinkML schemas
    Generate {
        /// Schema directory (default: src/schemas)
        #[arg(short, long, default_value = "src/schemas")]
        schema_dir: PathBuf,
        
        /// Output directory (default: src/generated)
        #[arg(short, long, default_value = "src/generated")]
        output_dir: PathBuf,
        
        /// Include patterns
        #[arg(short, long)]
        include: Vec<String>,
        
        /// Exclude patterns
        #[arg(short = 'x', long)]
        exclude: Vec<String>,
        
        /// Add serde derives
        #[arg(long, default_value = "true")]
        serde: bool,
        
        /// Add Debug derive
        #[arg(long, default_value = "true")]
        debug: bool,
        
        /// Add Clone derive
        #[arg(long, default_value = "true")]
        clone: bool,
        
        /// Validate before generating
        #[arg(long, default_value = "true")]
        validate_first: bool,
        
        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },
    
    /// Initialize LinkML configuration
    Init {
        /// Force overwrite existing configuration
        #[arg(short, long)]
        force: bool,
    },
    
    /// Format LinkML schemas
    Format {
        /// Schema directory (default: src/schemas)
        #[arg(short, long, default_value = "src/schemas")]
        schema_dir: PathBuf,
        
        /// Include patterns
        #[arg(short, long)]
        include: Vec<String>,
        
        /// Exclude patterns
        #[arg(short = 'x', long)]
        exclude: Vec<String>,
        
        /// Format in place
        #[arg(long, default_value = "true")]
        in_place: bool,
        
        /// Check only (don't modify files)
        #[arg(long)]
        check: bool,
    },
    
    /// Convert schemas to other formats
    Convert {
        /// Schema directory (default: src/schemas)
        #[arg(short, long, default_value = "src/schemas")]
        schema_dir: PathBuf,
        
        /// Output directory
        #[arg(short, long, required = true)]
        output_dir: PathBuf,
        
        /// Target format (json, jsonld, rdf, ttl)
        #[arg(short, long, required = true)]
        target: String,
        
        /// Include patterns
        #[arg(short, long)]
        include: Vec<String>,
        
        /// Exclude patterns
        #[arg(short = 'x', long)]
        exclude: Vec<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let Cargo::Linkml(cmd) = Cargo::parse();
    
    // Check if linkml executable is available
    check_linkml_executable()?;
    
    match cmd {
        LinkMLCommand::Validate {
            schema_dir,
            include,
            exclude,
            fail_on_error,
            verbose,
        } => {
            validator::validate_schemas(
                &schema_dir,
                &include,
                &exclude,
                fail_on_error,
                verbose,
            ).await
        }
        
        LinkMLCommand::Generate {
            schema_dir,
            output_dir,
            include,
            exclude,
            serde,
            debug,
            clone,
            validate_first,
            verbose,
        } => {
            generator::generate_code(
                &schema_dir,
                &output_dir,
                &include,
                &exclude,
                generator::GenerateOptions {
                    serde,
                    debug,
                    clone,
                    validate_first,
                    verbose,
                },
            ).await
        }
        
        LinkMLCommand::Init { force } => {
            init_config(force).await
        }
        
        LinkMLCommand::Format {
            schema_dir,
            include,
            exclude,
            in_place,
            check,
        } => {
            format_schemas(
                &schema_dir,
                &include,
                &exclude,
                in_place,
                check,
            ).await
        }
        
        LinkMLCommand::Convert {
            schema_dir,
            output_dir,
            target,
            include,
            exclude,
        } => {
            convert_schemas(
                &schema_dir,
                &output_dir,
                &target,
                &include,
                &exclude,
            ).await
        }
    }
}

/// Check if linkml executable is available
fn check_linkml_executable() -> Result<()> {
    which::which("linkml")
        .context("LinkML executable not found. Please install LinkML first.")?;
    Ok(())
}

/// Initialize LinkML configuration
async fn init_config(force: bool) -> Result<()> {
    let config_path = PathBuf::from("linkml.toml");
    
    if config_path.exists() && !force {
        eprintln!("{} Configuration file already exists. Use --force to overwrite.", "Error:".red());
        std::process::exit(1);
    }
    
    let config = LinkMLConfig::default();
    let toml = toml::to_string_pretty(&config)?;
    
    std::fs::write(&config_path, toml)?;
    
    println!("{} Created linkml.toml configuration file", "Success:".green());
    println!("\nNext steps:");
    println!("  1. Place your LinkML schemas in src/schemas/");
    println!("  2. Run `cargo linkml validate` to validate schemas");
    println!("  3. Run `cargo linkml generate` to generate Rust code");
    
    // Create schema directory if it doesn't exist
    let schema_dir = PathBuf::from("src/schemas");
    if !schema_dir.exists() {
        std::fs::create_dir_all(&schema_dir)?;
        println!("\n{} Created src/schemas/ directory", "Info:".blue());
    }
    
    Ok(())
}

/// Format LinkML schemas
async fn format_schemas(
    schema_dir: &Path,
    include: &[String],
    exclude: &[String],
    in_place: bool,
    check: bool,
) -> Result<()> {
    let schemas = find_schemas(schema_dir, include, exclude)?;
    
    if schemas.is_empty() {
        println!("{} No LinkML schemas found", "Info:".blue());
        return Ok(());
    }
    
    println!("Formatting {} schema(s)...", schemas.len());
    
    let pb = ProgressBar::new(schemas.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );
    
    let mut error_count = 0;
    
    for schema in &schemas {
        let relative_path = schema.strip_prefix(schema_dir).unwrap_or(schema);
        pb.set_message(format!("Formatting {}", relative_path.display()));
        
        let result = format_schema(schema, in_place, check).await;
        
        match result {
            Ok(changed) => {
                if check && changed {
                    eprintln!("{} {} would be reformatted", "!".yellow(), relative_path.display());
                    error_count += 1;
                } else if !check {
                    println!("{} Formatted {}", "✓".green(), relative_path.display());
                }
            }
            Err(e) => {
                eprintln!("{} Failed to format {}: {}", "✗".red(), relative_path.display(), e);
                error_count += 1;
            }
        }
        
        pb.inc(1);
    }
    
    pb.finish_with_message("Done");
    
    if check && error_count > 0 {
        eprintln!("\n{} {} file(s) would be reformatted", "Error:".red(), error_count);
        std::process::exit(1);
    } else if error_count > 0 {
        eprintln!("\n{} {} error(s) occurred", "Error:".red(), error_count);
        std::process::exit(1);
    }
    
    Ok(())
}

/// Format a single schema
async fn format_schema(schema: &Path, in_place: bool, check: bool) -> Result<bool> {
    let mut cmd = Command::new("linkml");
    cmd.arg("format");
    
    if check {
        cmd.arg("--check");
    } else if in_place {
        cmd.arg("--in-place");
    }
    
    cmd.arg(schema);
    
    let output = cmd.output()?;
    
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Format failed: {}", error);
    }
    
    // If checking, look for differences
    if check {
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(!stdout.is_empty())
    } else {
        Ok(false)
    }
}

/// Convert schemas to another format
async fn convert_schemas(
    schema_dir: &Path,
    output_dir: &Path,
    target: &str,
    include: &[String],
    exclude: &[String],
) -> Result<()> {
    let schemas = find_schemas(schema_dir, include, exclude)?;
    
    if schemas.is_empty() {
        println!("{} No LinkML schemas found", "Info:".blue());
        return Ok(());
    }
    
    // Validate target format
    let valid_formats = ["json", "jsonld", "rdf", "ttl"];
    if !valid_formats.contains(&target) {
        eprintln!("{} Invalid target format: {}", "Error:".red(), target);
        eprintln!("Valid formats: {}", valid_formats.join(", "));
        std::process::exit(1);
    }
    
    // Create output directory
    std::fs::create_dir_all(output_dir)?;
    
    println!("Converting {} schema(s) to {} format...", schemas.len(), target);
    
    let pb = ProgressBar::new(schemas.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );
    
    let mut success_count = 0;
    
    for schema in &schemas {
        let relative_path = schema.strip_prefix(schema_dir).unwrap_or(schema);
        pb.set_message(format!("Converting {}", relative_path.display()));
        
        let result = convert_schema(schema, output_dir, target).await;
        
        match result {
            Ok(output_file) => {
                println!("{} Converted {} → {}", "✓".green(), relative_path.display(), output_file.display());
                success_count += 1;
            }
            Err(e) => {
                eprintln!("{} Failed to convert {}: {}", "✗".red(), relative_path.display(), e);
            }
        }
        
        pb.inc(1);
    }
    
    pb.finish_with_message("Done");
    
    println!("\n{} Converted {}/{} schemas successfully", 
             "Summary:".bold(), 
             success_count, 
             schemas.len());
    
    Ok(())
}

/// Convert a single schema
async fn convert_schema(schema: &Path, output_dir: &Path, target: &str) -> Result<PathBuf> {
    let stem = schema.file_stem()
        .and_then(|s| s.to_str())
        .context("Invalid schema filename")?;
    
    let output_file = output_dir.join(format!("{}.{}", stem, target));
    
    let mut cmd = Command::new("linkml");
    cmd.arg("convert");
    cmd.arg("-f").arg(target);
    cmd.arg("-o").arg(&output_file);
    cmd.arg(schema);
    
    let output = cmd.output()?;
    
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Conversion failed: {}", error);
    }
    
    Ok(output_file)
}

/// Find schema files based on patterns
fn find_schemas(
    schema_dir: &Path,
    include: &[String],
    exclude: &[String],
) -> Result<Vec<PathBuf>> {
    let default_patterns = vec![
        "**/*.linkml.yaml".to_string(),
        "**/*.linkml.yml".to_string(),
        "**/*.linkml".to_string(),
    ];
    
    let patterns = if include.is_empty() {
        &default_patterns
    } else {
        include
    };
    
    let mut schemas = Vec::new();
    
    for entry in WalkDir::new(schema_dir) {
        let entry = entry?;
        let path = entry.path();
        
        if !path.is_file() {
            continue;
        }
        
        let relative = path.strip_prefix(schema_dir).unwrap_or(path);
        let relative_str = relative.to_string_lossy();
        
        // Check if matches any include pattern
        let matches_include = patterns.iter().any(|pattern| {
            glob::Pattern::new(pattern)
                .map(|p| p.matches(&relative_str))
                .unwrap_or(false)
        });
        
        if !matches_include {
            continue;
        }
        
        // Check if matches any exclude pattern
        let matches_exclude = exclude.iter().any(|pattern| {
            glob::Pattern::new(pattern)
                .map(|p| p.matches(&relative_str))
                .unwrap_or(false)
        });
        
        if !matches_exclude {
            schemas.push(path.to_path_buf());
        }
    }
    
    schemas.sort();
    Ok(schemas)
}