//! Enhanced CLI commands for `LinkML`
//!
//! This module provides the complete set of `LinkML` command-line tools
//! matching Python `LinkML` functionality.

mod app;
mod types;

pub use app::LinkMLApp;
pub use types::{
    AuthType, ConflictResolution, DiffFormat, DumpFormat, LinkMLCli, LinkMLCommand, LintFormat,
    LoadFormat, MergeStrategy, OutputFormat, SchemaFormat,
};

/// Main entry point for the enhanced CLI
pub async fn run() -> linkml_core::error::Result<()> {
    let app = LinkMLApp::from_args();
    app.run().await
}