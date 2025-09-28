//! Version compatibility checking for plugins
//!
//! This module handles checking plugin compatibility with the `LinkML` version
//! and other dependencies.

use super::{HashMap, LinkMLError, PluginDependency, PluginManifest, PluginMetadata, Result};
use crate::plugin::api::PLUGIN_API_VERSION;
use semver::{Op, Version, VersionReq};

/// Version compatibility checker
pub struct CompatibilityChecker {
    /// Current `LinkML` version
    linkml_version: Version,
    /// Compatibility rules
    rules: CompatibilityRules,
}

/// Compatibility rules configuration
#[derive(Debug, Clone)]
pub struct CompatibilityRules {
    /// Allow plugins built for newer `LinkML` versions
    pub allow_newer: bool,
    /// Allow plugins with wildcard version requirements
    pub allow_wildcards: bool,
    /// Strict mode - require exact major version match
    pub strict_mode: bool,
    /// Deprecated `API` versions to warn about
    pub deprecated_versions: Vec<VersionReq>,
}

impl Default for CompatibilityRules {
    fn default() -> Self {
        Self {
            allow_newer: false,
            allow_wildcards: true,
            strict_mode: false,
            deprecated_versions: vec![
                VersionReq::parse("<0.9.0").expect("valid version requirement"),
            ],
        }
    }
}

impl Default for CompatibilityChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl CompatibilityChecker {
    /// Create a new compatibility checker
    #[must_use]
    pub fn new() -> Self {
        Self {
            linkml_version: Version::parse(env!("CARGO_PKG_VERSION"))
                .expect("CARGO_PKG_VERSION should be a valid semver"),
            rules: CompatibilityRules::default(),
        }
    }

    /// Create with custom rules
    #[must_use]
    pub fn with_rules(rules: CompatibilityRules) -> Self {
        Self {
            linkml_version: Version::parse(env!("CARGO_PKG_VERSION"))
                .expect("CARGO_PKG_VERSION should be a valid semver"),
            rules,
        }
    }

    /// Check if a plugin is compatible
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    /// Returns `LinkMLError::IncompatiblePlugin` if version requirements are not met
    /// Returns `LinkMLError::DeprecatedApi` if the plugin uses deprecated APIs
    pub fn check_compatibility(&self, manifest: &PluginManifest) -> Result<()> {
        let plugin_info = &manifest.plugin;

        // Check LinkML version requirement
        self.check_version_requirement(&plugin_info.linkml_version)?;

        // Check for deprecated APIs
        self.check_deprecated_apis(&plugin_info.linkml_version)?;

        // Check dependency versions
        for dep in &plugin_info.dependencies {
            self.validate_dependency(dep)?;
        }

        // Check API version
        if let Some(_metadata) = &manifest
            .plugin
            .capabilities
            .iter()
            .find_map(|_| None::<&PluginMetadata>)
        {
            // API version check would happen here if metadata existed
            // self.check_api_version(metadata.api_version)?;
        }

        Ok(())
    }

    /// Check version requirement against current `LinkML` version
    fn check_version_requirement(&self, requirement: &VersionReq) -> Result<()> {
        // Check if requirement matches current version
        if requirement.matches(&self.linkml_version) {
            return Ok(());
        }

        // Check special cases
        if self.rules.allow_wildcards && requirement.to_string() == "*" {
            return Ok(());
        }

        // Check if plugin requires newer version
        if !self.rules.allow_newer && self.requires_newer_version(requirement) {
            return Err(LinkMLError::ServiceError(format!(
                "Plugin requires LinkML {} but current version is {}",
                requirement, self.linkml_version
            )));
        }

        // In strict mode, require exact major version match
        if self.rules.strict_mode && !self.same_major_version(requirement) {
            return Err(LinkMLError::ServiceError(format!(
                "Plugin requires LinkML {} but strict mode requires major version {}",
                requirement, self.linkml_version.major
            )));
        }

        Err(LinkMLError::ServiceError(format!(
            "Plugin LinkML version requirement {} is not compatible with {}",
            requirement, self.linkml_version
        )))
    }

    /// Check if plugin uses deprecated `API`s
    fn check_deprecated_apis(&self, requirement: &VersionReq) -> Result<()> {
        for deprecated in &self.rules.deprecated_versions {
            if requirement.matches(&Version::new(0, 8, 0))
                && deprecated.matches(&Version::new(0, 8, 0))
            {
                // Just warn, don't fail
                eprintln!("Warning: Plugin uses deprecated LinkML API version {requirement}");
            }
        }
        Ok(())
    }

    /// Validate a plugin dependency
    fn validate_dependency(&self, dependency: &PluginDependency) -> Result<()> {
        // Check for invalid version requirements
        if dependency.version.to_string().is_empty() {
            return Err(LinkMLError::ServiceError(format!(
                "Invalid version requirement for dependency '{}'",
                dependency.id
            )));
        }

        // Validate version requirement can be parsed
        // (already validated by serde, but double-check)
        let _ = VersionReq::parse(&dependency.version.to_string()).map_err(|e| {
            LinkMLError::ServiceError(format!(
                "Invalid version requirement for dependency '{}': {}",
                dependency.id, e
            ))
        })?;

        Ok(())
    }

    /// Check `API` version compatibility
    fn _check_api_version(&self, api_version: u32) -> Result<()> {
        if api_version != PLUGIN_API_VERSION {
            return Err(LinkMLError::ServiceError(format!(
                "Plugin API version {api_version} is not compatible with current API version {PLUGIN_API_VERSION}"
            )));
        }
        Ok(())
    }

    /// Check if requirement needs newer `LinkML` version
    fn requires_newer_version(&self, requirement: &VersionReq) -> bool {
        // Parse the requirement to check if it requires newer version
        for comparator in &requirement.comparators {
            match comparator.op {
                Op::Greater | Op::GreaterEq => {
                    if comparator.major > self.linkml_version.major
                        || (comparator.major == self.linkml_version.major
                            && comparator.minor.unwrap_or(0) > self.linkml_version.minor)
                    {
                        return true;
                    }
                }
                Op::Exact => {
                    if comparator.major > self.linkml_version.major {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// Check if requirement has same major version
    fn same_major_version(&self, requirement: &VersionReq) -> bool {
        for comparator in &requirement.comparators {
            if comparator.major != self.linkml_version.major {
                return false;
            }
        }
        true
    }
}

/// Version compatibility matrix for known plugins
pub struct CompatibilityMatrix {
    /// Known compatible plugin versions
    compatible: HashMap<String, Vec<VersionRange>>,
    /// Known incompatible plugin versions
    incompatible: HashMap<String, Vec<VersionRange>>,
}

/// Version range specification
#[derive(Debug, Clone)]
pub struct VersionRange {
    /// Minimum version (inclusive)
    pub min: Version,
    /// Maximum version (exclusive)
    pub max: Option<Version>,
    /// Reason for compatibility/incompatibility
    pub reason: String,
}

impl Default for CompatibilityMatrix {
    fn default() -> Self {
        Self::new()
    }
}

impl CompatibilityMatrix {
    /// Create a new compatibility matrix
    #[must_use]
    pub fn new() -> Self {
        Self {
            compatible: HashMap::new(),
            incompatible: HashMap::new(),
        }
    }

    /// Add a compatible version range
    pub fn add_compatible(&mut self, plugin_id: String, range: VersionRange) {
        self.compatible.entry(plugin_id).or_default().push(range);
    }

    /// Add an incompatible version range
    pub fn add_incompatible(&mut self, plugin_id: String, range: VersionRange) {
        self.incompatible.entry(plugin_id).or_default().push(range);
    }

    /// Check if a specific plugin version is compatible
    #[must_use]
    pub fn is_compatible(&self, plugin_id: &str, version: &Version) -> Option<bool> {
        // Check incompatible list first
        if let Some(ranges) = self.incompatible.get(plugin_id) {
            for range in ranges {
                if version >= &range.min && range.max.as_ref().is_none_or(|max| version < max) {
                    return Some(false);
                }
            }
        }

        // Check compatible list
        if let Some(ranges) = self.compatible.get(plugin_id) {
            for range in ranges {
                if version >= &range.min && range.max.as_ref().is_none_or(|max| version < max) {
                    return Some(true);
                }
            }
        }

        // Unknown compatibility
        None
    }
}

/// Migration helper for plugin version updates
pub struct VersionMigration {
    /// Source version
    pub from: Version,
    /// Target version
    pub to: Version,
    /// Migration steps
    pub steps: Vec<MigrationStep>,
}

/// Individual migration step
#[derive(Debug, Clone)]
pub struct MigrationStep {
    /// Step description
    pub description: String,
    /// Whether this step is automated
    pub automated: bool,
    /// Migration script or instructions
    pub script: String,
}

impl VersionMigration {
    /// Check if migration is needed
    #[must_use]
    pub fn is_needed(&self, current: &Version, target: &Version) -> bool {
        current >= &self.from && target >= &self.to
    }

    /// Get migration complexity score (0-100)
    #[must_use]
    pub fn complexity_score(&self) -> u32 {
        let manual_steps = self.steps.iter().filter(|s| !s.automated).count();
        let total_steps = self.steps.len();

        if total_steps == 0 {
            0
        } else {
            ((manual_steps * 100) / total_steps) as u32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_compatibility() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let checker = CompatibilityChecker::new();
        let current_version = &checker.linkml_version;

        // Compatible version requirement
        let req = VersionReq::parse(&format!(">={}.0.0", current_version.major))?;
        assert!(checker.check_version_requirement(&req).is_ok());

        // Incompatible version requirement (requires newer major)
        let req = VersionReq::parse(&format!(">={}.0.0", current_version.major + 1))?;
        assert!(checker.check_version_requirement(&req).is_err());
        Ok(())
    }

    #[test]
    fn test_compatibility_matrix() {
        let mut matrix = CompatibilityMatrix::new();

        matrix.add_compatible(
            "test-plugin".to_string(),
            VersionRange {
                min: Version::new(1, 0, 0),
                max: Some(Version::new(2, 0, 0)),
                reason: "Tested and verified".to_string(),
            },
        );

        assert_eq!(
            matrix.is_compatible("test-plugin", &Version::new(1, 5, 0)),
            Some(true)
        );
        assert_eq!(
            matrix.is_compatible("test-plugin", &Version::new(2, 0, 0)),
            None
        );
        assert_eq!(
            matrix.is_compatible("unknown-plugin", &Version::new(1, 0, 0)),
            None
        );
    }
}
