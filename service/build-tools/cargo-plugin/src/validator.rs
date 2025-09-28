//! Schema validation functionality

use anyhow::Result;
use colored::Colorize;
use std::path::Path;
use std::process::Command;

/// Validate LinkML schemas
pub async fn validate_schemas(
    schema_dir: &Path,
    include: &[String],
    exclude: &[String],
    fail_on_error: bool,
    verbose: bool,
) -> Result<()> {
    let schemas = crate::find_schemas(schema_dir, include, exclude)?;

    if schemas.is_empty() {
        println!("{} No LinkML schemas found", "Info:".blue());
        return Ok(());
    }

    println!("Validating {} schema(s)...", schemas.len());

    let mut error_count = 0;
    let mut warning_count = 0;

    for schema in &schemas {
        let relative_path = schema.strip_prefix(schema_dir).unwrap_or(schema);

        match validate_schema(schema, verbose).await {
            Ok(ValidationResult { warnings }) => {
                println!("{} Valid: {}", "✓".green(), relative_path.display());
                if !warnings.is_empty() {
                    warning_count += warnings.len();
                    for warning in warnings {
                        println!("  {} {}", "⚠".yellow(), warning);
                    }
                }
            }
            Err(e) => {
                error_count += 1;
                eprintln!("{} Invalid: {}", "✗".red(), relative_path.display());
                eprintln!("  {}", e);
            }
        }
    }

    // Summary
    println!("
{}", "Summary:".bold());
    println!("  Total schemas: {}", schemas.len());
    println!("  Valid: {}", (schemas.len() - error_count).to_string().green());
    if error_count > 0 {
        println!("  Invalid: {}", error_count.to_string().red());
    }
    if warning_count > 0 {
        println!("  Warnings: {}", warning_count.to_string().yellow());
    }

    if error_count > 0 && fail_on_error {
        anyhow::bail!("Validation failed for {} schema(s)", error_count);
    }

    Ok(())
}

/// Result of schema validation
struct ValidationResult {
    warnings: Vec<String>,
}

/// Validate a single schema
async fn validate_schema(schema: &Path, verbose: bool) -> Result<ValidationResult> {
    let mut cmd = Command::new("linkml");
    cmd.arg("validate");

    if verbose {
        cmd.arg("--verbose");
    }

    cmd.arg(schema);

    let output = cmd.output()?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("{}", error.trim());
    }

    // Parse warnings from output
    let stdout = String::from_utf8_lossy(&output.stdout);
    let warnings: Vec<String> = stdout
        .lines()
        .filter(|line| line.contains("WARNING") || line.contains("Warning"))
        .map(|line| line.trim().to_string())
        .collect();

    Ok(ValidationResult { warnings })
}
