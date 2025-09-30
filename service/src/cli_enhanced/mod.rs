//! Enhanced CLI commands for `LinkML`.
//!
//! This module provides the complete set of `LinkML` command-line tools
//! matching Python `LinkML` functionality.

mod app;
pub mod commands;
mod types;

pub use app::LinkMLApp;
pub use types::{
    AuthType, ConflictResolution, DiffFormat, DumpFormat, LinkMLCli, LinkMLCommand, LintFormat,
    LoadFormat, MergeStrategy, OutputFormat, SchemaFormat,
};

/// Main entry point for the enhanced CLI
///
/// # Errors
/// Returns error if CLI execution fails or encounters invalid arguments.
pub async fn run() -> linkml_core::error::Result<()> {
    use timestamp_core::factory::create_timestamp_service;
    let timestamp_service = create_timestamp_service();
    let app = LinkMLApp::from_args_with_timestamp(timestamp_service);
    app.run().await
}
