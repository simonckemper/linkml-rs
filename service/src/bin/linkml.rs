//! `LinkML` command-line interface
//!
//! This binary provides the `linkml` command-line tool for working with
//! `LinkML` schemas and data.

use linkml_core::error::Result;
use linkml_service::cli_enhanced;

#[tokio::main]
async fn main() -> Result<()> {
    // Run the enhanced CLI
    cli_enhanced::run().await
}
