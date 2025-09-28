//! Schema version management
//!
//! Handles schema versioning, version comparison, and checksum generation.

use chrono::{DateTime, Utc};
use std::fmt::Write;
use linkml_core::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::cmp::Ordering;
use std::fmt;

use super::{MigrationError, MigrationResult};

/// Semantic version for schemas
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaVersion {
    /// Major version (breaking changes)
    pub major: u32,
    /// Minor version (new features)
    pub minor: u32,
    /// Patch version (bug fixes)
    pub patch: u32,
    /// Version timestamp
    pub timestamp: DateTime<Utc>,
    /// Schema content checksum
    pub checksum: String}

impl SchemaVersion {
    /// Create a new schema version
    #[must_use] pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
            timestamp: DateTime::<Utc>::from_timestamp(0, 0)
                .unwrap_or_else(|| DateTime::<Utc>::from_timestamp(946684800, 0)
                    .expect("Valid timestamp for 2000-01-01")), // 2000-01-01
            checksum: String::new()}
    }

    /// Parse version from string (e.g., "1.2.3")
    pub fn parse(version_str: &str) -> MigrationResult<Self> {
        let parts: Vec<&str> = version_str.split('.').collect();
        if parts.len() != 3 {
            return Err(MigrationError::InvalidVersion(
                format!("Expected format: major.minor.patch, got: {version_str}")
            ));
        }

        let major = parts[0].parse::<u32>()
            .map_err(|_| MigrationError::InvalidVersion("Invalid major version".to_string()))?;
        let minor = parts[1].parse::<u32>()
            .map_err(|_| MigrationError::InvalidVersion("Invalid minor version".to_string()))?;
        let patch = parts[2].parse::<u32>()
            .map_err(|_| MigrationError::InvalidVersion("Invalid patch version".to_string()))?;

        Ok(Self::new(major, minor, patch))
    }

    /// Generate checksum for schema content
    #[must_use] pub fn calculate_checksum(schema: &SchemaDefinition) -> String {
        let mut hasher = Sha256::new();

        // Hash schema name and description
        hasher.update(schema.name.as_bytes());
        if let Some(desc) = &schema.description {
            hasher.update(desc.as_bytes());
        }

        // Hash classes in sorted order
        let mut class_names: Vec<&String> = schema.classes.keys().collect();
        class_names.sort();
        for name in class_names {
            hasher.update(name.as_bytes());
            if let Some(class) = schema.classes.get(name) {
                // Hash class properties
                hasher.update(format!("{class:?}").as_bytes());
            }
        }

        // Hash slots in sorted order
        let mut slot_names: Vec<&String> = schema.slots.keys().collect();
        slot_names.sort();
        for name in slot_names {
            hasher.update(name.as_bytes());
            if let Some(slot) = schema.slots.get(name) {
                hasher.update(format!("{slot:?}").as_bytes());
            }
        }

        // Convert to hex string
        format!("{:x}", hasher.finalize())
    }

    /// Check if this version is newer than another
    #[must_use] pub fn is_newer_than(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Greater
    }

    /// Check if this is a breaking change from another version
    #[must_use] pub fn is_breaking_change_from(&self, other: &Self) -> bool {
        self.major > other.major
    }

    /// Check if this is a feature addition from another version
    #[must_use] pub fn is_feature_addition_from(&self, other: &Self) -> bool {
        self.major == other.major && self.minor > other.minor
    }

    /// Check if this is a patch from another version
    #[must_use] pub fn is_patch_from(&self, other: &Self) -> bool {
        self.major == other.major && self.minor == other.minor && self.patch > other.patch
    }
}

impl fmt::Display for SchemaVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl Ord for SchemaVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.major.cmp(&other.major) {
            Ordering::Equal => match self.minor.cmp(&other.minor) {
                Ordering::Equal => self.patch.cmp(&other.patch),
                other => other},
            other => other}
    }
}

impl PartialOrd for SchemaVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A schema with version information
#[derive(Debug, Clone)]
pub struct VersionedSchema {
    pub schema: SchemaDefinition,
    /// Version information
    pub version: SchemaVersion}

impl VersionedSchema {
    /// Create a new versioned schema
        let mut version = version;
        version.checksum = SchemaVersion::calculate_checksum(&schema);
        Self { schema, version }
    }

    /// Create from schema with version string
    pub fn from_schema(schema: SchemaDefinition, version_str: &str) -> MigrationResult<Self> {
        let version = SchemaVersion::parse(version_str)?;
        Ok(Self::new(schema, version))
    }

    /// Extract version from schema metadata if available
        // Look for version in schema metadata
        // This could be in annotations or a special field
        schema.version.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let v = SchemaVersion::parse("1.2.3").expect("should parse valid version: {}");
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);

        assert!(SchemaVersion::parse("1.2").is_err());
        assert!(SchemaVersion::parse("1.2.3.4").is_err());
        assert!(SchemaVersion::parse("a.b.c").is_err());
        Ok(())
    }

    #[test]
    fn test_version_comparison() {
        let v1 = SchemaVersion::new(1, 0, 0);
        let v2 = SchemaVersion::new(1, 1, 0);
        let v3 = SchemaVersion::new(2, 0, 0);

        assert!(v2.is_newer_than(&v1));
        assert!(v3.is_newer_than(&v2));
        assert!(!v1.is_newer_than(&v2));

        assert!(v3.is_breaking_change_from(&v1));
        assert!(!v2.is_breaking_change_from(&v1));

        assert!(v2.is_feature_addition_from(&v1));
        assert!(!v3.is_feature_addition_from(&v1));
    }

    #[test]
    fn test_checksum_generation() {
        let mut schema1 = SchemaDefinition::default();
        schema1.name = "TestSchema".to_string();

        let mut schema2 = SchemaDefinition::default();
        schema2.name = "TestSchema".to_string();

        let checksum1 = SchemaVersion::calculate_checksum(&schema1);
        let checksum2 = SchemaVersion::calculate_checksum(&schema2);

        assert_eq!(checksum1, checksum2);

        // Change schema
        schema2.name = "DifferentSchema".to_string();
        let checksum3 = SchemaVersion::calculate_checksum(&schema2);
        assert_ne!(checksum1, checksum3);
    }
}