//! Example showing how to update CLI commands to use file system adapter
//!
//! This demonstrates the pattern for migrating from direct std::fs usage
//! to the FileSystemOperations trait.

use linkml_service::{
    cli_fs_adapter::{CLIFileSystemOps, default_cli_fs},
    file_system_adapter::TokioFileSystemAdapter,
};
use std::path::Path;
use std::sync::Arc;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Example 1: Using default file system adapter
    example_with_default_fs().await?;

    // Example 2: Using sandboxed file system adapter
    example_with_sandboxed_fs().await?;

    // Example 3: Migrating existing CLI code
    example_migration().await?;

    // Example 4: Command pattern with file system adapter
    example_command_pattern().await?;

    Ok(())
}

/// Example using default file system adapter (unrestricted)
async fn example_with_default_fs() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("=== Example 1: Default File System Adapter ===");

    let cli_fs = default_cli_fs();

    // Write a file
    let output_path = Path::new("output/example.yaml");
    let content = "name: ExampleSchema
classes:
  Person:
    attributes:
      name: string
";

    cli_fs.write_output(output_path, content).await?;
    println!("✓ Wrote file to {}", output_path.display());

    // Read it back
    let read_content = cli_fs.read_input(output_path).await?;
    println!("✓ Read {} bytes back", read_content.len());

    // Check existence
    let exists = cli_fs.exists(output_path).await?;
    println!("✓ File exists: {}", exists);

    Ok(())
}

/// Example using sandboxed file system adapter
async fn example_with_sandboxed_fs() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!(
        "
=== Example 2: Sandboxed File System Adapter ==="
    );

    use tempfile::TempDir;
    let temp_dir = TempDir::new()?;

    // Create sandboxed adapter limited to temp directory
    let fs = Arc::new(TokioFileSystemAdapter::sandboxed(
        temp_dir.path().to_path_buf(),
    ));
    let cli_fs = CLIFileSystemOps::new(fs);

    // This will work - within sandbox
    let safe_path = Path::new("data/schema.yaml");
    cli_fs.write_output(safe_path, "safe content").await?;
    println!("✓ Wrote file within sandbox");

    // This would fail - escapes sandbox
    let unsafe_path = Path::new("../escape.yaml");
    match cli_fs.write_output(unsafe_path, "escape attempt").await {
        Err(e) => println!("✓ Correctly blocked sandbox escape: {}", e),
        Ok(_) => println!("✗ ERROR: Sandbox escape should have failed!"),
    }

    Ok(())
}

/// Example showing how to migrate existing CLI code
async fn example_migration() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!(
        "
=== Example 3: Migrating CLI Code ==="
    );

    // OLD CODE (direct file system):
    // std::fs::create_dir_all(output.parent().unwrap_or(Path::new(".")))?;
    // std::fs::write(output, result)?;

    // NEW CODE (with adapter):
    let cli_fs = default_cli_fs();
    let output = Path::new("output/migrated/result.json");
    let result = r#"{"name": "Example", "version": "1.0"}"#;

    // The adapter handles parent directory creation automatically
    cli_fs.write_output(output, result).await?;
    println!("✓ Migrated code successfully wrote output");

    // More examples of migration patterns:

    // OLD: let content = std::fs::read_to_string(path)?;
    // NEW:
    let content = cli_fs.read_input(output).await?;
    println!("✓ Read {} bytes", content.len());

    // OLD: if !path.exists() { /* ... */ }
    // NEW:
    if !cli_fs.exists(output).await? {
        println!("File doesn't exist");
    } else {
        println!("✓ File exists check works");
    }

    // OLD: std::fs::remove_file(path)?;
    // NEW:
    cli_fs.remove_file(output).await?;
    println!("✓ File removed");

    Ok(())
}

/// Example demonstrating command pattern with file system adapter
async fn example_command_pattern() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!(
        "
=== Example 4: Command Pattern ==="
    );

    let cli_fs = default_cli_fs();
    let schema_path = Path::new("output/example.yaml");
    let output_path = Path::new("output/generated_result.txt");

    // Use the command example function
    generate_command_example(schema_path, output_path, &cli_fs).await?;

    Ok(())
}

/// Example of how to update a CLI command function
async fn generate_command_example(
    schema_path: &Path,
    output_path: &Path,
    cli_fs: &CLIFileSystemOps<impl linkml_service::file_system_adapter::FileSystemOperations>,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Load schema using the adapter
    let schema_content = cli_fs.read_input(schema_path).await?;

    // Validate schema content is not empty
    if schema_content.trim().is_empty() {
        return Err("Schema file is empty".into());
    }

    // Process schema - include content size in output for verification
    let result = format!(
        "Generated from: {}\nSchema size: {} bytes\nFirst 100 chars: {}",
        schema_path.display(),
        schema_content.len(),
        schema_content.chars().take(100).collect::<String>()
    );

    // Write output using the adapter
    cli_fs.write_output(output_path, &result).await?;

    println!("✓ Generated output to {}", output_path.display());
    Ok(())
}
