//! Schema loader for loading schemas from files and URLs

use linkml_core::{
    error::{LinkMLError, Result},
    settings::ImportSettings,
    types::SchemaDefinition,
};
use reqwest;
use std::path::{Path, PathBuf};
use tokio::fs;

use super::{ImportResolverV2, Parser};

/// Loader for `LinkML` schemas from various sources
pub struct SchemaLoader {
    parser: Parser,
    http_client: reqwest::Client,
}

impl SchemaLoader {
    /// Create a new schema loader
    #[must_use]
    pub fn new() -> Self {
        Self {
            parser: Parser::new(),
            http_client: reqwest::Client::new(),
        }
    }

    /// Load a schema from a file path
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub async fn load_file(&self, path: impl AsRef<Path>) -> Result<SchemaDefinition> {
        let path = path.as_ref();

        // Read file content
        let content = fs::read_to_string(path)
            .await
            .map_err(|e| LinkMLError::service(format!("Failed to read file: {e}")))?;

        // Determine format from extension
        let extension = path
            .extension()
            .and_then(|s| s.to_str())
            .ok_or_else(|| LinkMLError::parse("No file extension found"))?;

        // Parse the schema
        let schema = self.parser.parse_str(&content, extension)?;

        // Set up import settings with the file's parent directory as search path
        let mut settings = ImportSettings::default();
        if let Some(parent) = path.parent() {
            settings
                .search_paths
                .push(parent.to_string_lossy().to_string());
        }

        // Use schema settings if available
        if let Some(schema_settings) = &schema.settings
            && let Some(import_settings) = &schema_settings.imports
        {
            settings = import_settings.clone();

            // Resolve relative search paths from schema settings
            if let Some(parent) = path.parent() {
                // Make relative paths absolute based on schema location
                settings.search_paths = settings
                    .search_paths
                    .iter()
                    .map(|p| {
                        let path_buf = PathBuf::from(p);
                        if path_buf.is_relative() {
                            parent.join(path_buf).to_string_lossy().to_string()
                        } else {
                            p.clone()
                        }
                    })
                    .collect();

                // Also add the parent directory if not already present
                let parent_str = parent.to_string_lossy().to_string();
                if !settings.search_paths.contains(&parent_str) {
                    settings.search_paths.push(parent_str);
                }
            }
        }

        // Resolve imports using enhanced resolver
        let import_resolver = ImportResolverV2::with_settings(settings);
        import_resolver.resolve_imports(&schema).await
    }

    /// Load a schema from a `URL`
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub async fn load_url(&self, url: &str) -> Result<SchemaDefinition> {
        // Fetch content from URL
        let response = self
            .http_client
            .get(url)
            .send()
            .await
            .map_err(|e| LinkMLError::service(format!("Failed to fetch URL: {e}")))?;

        if !response.status().is_success() {
            return Err(LinkMLError::service(format!(
                "HTTP error {}: {}",
                response.status(),
                response.status().canonical_reason().unwrap_or("Unknown")
            )));
        }

        let content = response
            .text()
            .await
            .map_err(|e| LinkMLError::service(format!("Failed to read response: {e}")))?;

        // Determine format from URL extension or content type (case-insensitive)
        let url_lower = url.to_lowercase();
        let format = if std::path::Path::new(&url_lower)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
        {
            "json"
        } else if std::path::Path::new(&url_lower)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("yaml") || ext.eq_ignore_ascii_case("yml"))
        {
            "yaml"
        } else {
            // Default to YAML as it's more common for LinkML
            "yaml"
        };

        // Parse the schema
        let schema = self.parser.parse_str(&content, format)?;

        // Set up import settings with URL base
        let mut settings = ImportSettings::default();

        // Add base URL path for relative URL imports
        if let Ok(parsed_url) = url::Url::parse(url)
            && let Ok(base) = parsed_url.join("./")
        {
            settings.base_url = Some(base.to_string());
        }

        // Use schema settings if available
        if let Some(schema_settings) = &schema.settings
            && let Some(import_settings) = &schema_settings.imports
        {
            settings = import_settings.clone();
            // Still set base URL if not already set
            if settings.base_url.is_none()
                && let Ok(parsed_url) = url::Url::parse(url)
                && let Ok(base) = parsed_url.join("./")
            {
                settings.base_url = Some(base.to_string());
            }
        }

        // Resolve imports using enhanced resolver
        let import_resolver = ImportResolverV2::with_settings(settings);
        import_resolver.resolve_imports(&schema).await
    }

    /// Load a schema from a string with specified format
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub async fn load_string(&self, content: &str, format: &str) -> Result<SchemaDefinition> {
        let schema = self.parser.parse_str(content, format)?;

        // Use schema settings if available, otherwise defaults
        let settings = if let Some(schema_settings) = &schema.settings {
            schema_settings.imports.clone().unwrap_or_default()
        } else {
            ImportSettings::default()
        };

        // Resolve imports using enhanced resolver
        let import_resolver = ImportResolverV2::with_settings(settings);
        import_resolver.resolve_imports(&schema).await
    }
}

impl Default for SchemaLoader {
    fn default() -> Self {
        Self::new()
    }
}
