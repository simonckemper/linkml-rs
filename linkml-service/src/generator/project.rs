//! Project generator for LinkML schemas
//!
//! This module generates complete project scaffolding from LinkML schemas,
//! including directory structure, configuration files, build scripts, and documentation.

use crate::error::LinkMLError;
use crate::generator::traits::{Generator, GeneratorConfig};
use linkml_core::schema::Schema;
use std::collections::HashMap;
use std::path::PathBuf;

/// Project generator configuration
#[derive(Debug, Clone)]
pub struct ProjectGeneratorConfig {
    /// Base generator configuration
    pub base: GeneratorConfig,
    /// Project name (defaults to schema name)
    pub project_name: Option<String>,
    /// Target language/framework
    pub target: ProjectTarget,
    /// Include Docker configuration
    pub include_docker: bool,
    /// Include CI/CD configuration
    pub include_ci: bool,
    /// Include testing framework
    pub include_tests: bool,
    /// Include documentation
    pub include_docs: bool,
    /// Include examples
    pub include_examples: bool,
    /// License type
    pub license: LicenseType,
    /// Author information
    pub author: Option<String>,
    /// Author email
    pub author_email: Option<String>,
    /// Project version
    pub version: String,
}

/// Project target language/framework
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectTarget {
    /// Python project with Poetry
    Python,
    /// TypeScript/JavaScript project with npm
    TypeScript,
    /// Rust project with Cargo
    Rust,
    /// Java project with Maven
    Java,
    /// Go project with modules
    Go,
    /// Multi-language project
    MultiLanguage,
}

/// License type for the project
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LicenseType {
    /// MIT License
    Mit,
    /// Apache 2.0 License
    Apache2,
    /// BSD 3-Clause License
    Bsd3,
    /// GPL v3 License
    Gpl3,
    /// Creative Commons CC0
    Cc0,
    /// No license (proprietary)
    Proprietary,
}

impl Default for ProjectGeneratorConfig {
    fn default() -> Self {
        Self {
            base: GeneratorConfig::default(),
            project_name: None,
            target: ProjectTarget::Python,
            include_docker: true,
            include_ci: true,
            include_tests: true,
            include_docs: true,
            include_examples: true,
            license: LicenseType::Mit,
            author: None,
            author_email: None,
            version: "0.1.0".to_string(),
        }
    }
}

/// Project generator
pub struct ProjectGenerator {
    config: ProjectGeneratorConfig,
}

/// File to generate in the project
struct ProjectFile {
    path: String,
    content: String,
}

impl ProjectGenerator {
    /// Create a new project generator
    pub fn new(config: ProjectGeneratorConfig) -> Self {
        Self { config }
    }
    
    /// Generate project structure
    fn generate_project(&self, schema: &Schema) -> Result<String, LinkMLError> {
        let project_name = self.get_project_name(schema);
        let mut files = Vec::new();
        
        // Generate base structure
        files.extend(self.generate_base_structure(&project_name, schema)?);
        
        // Generate language-specific files
        match self.config.target {
            ProjectTarget::Python => files.extend(self.generate_python_project(&project_name, schema)?),
            ProjectTarget::TypeScript => files.extend(self.generate_typescript_project(&project_name, schema)?),
            ProjectTarget::Rust => files.extend(self.generate_rust_project(&project_name, schema)?),
            ProjectTarget::Java => files.extend(self.generate_java_project(&project_name, schema)?),
            ProjectTarget::Go => files.extend(self.generate_go_project(&project_name, schema)?),
            ProjectTarget::MultiLanguage => files.extend(self.generate_multi_language_project(&project_name, schema)?),
        }
        
        // Generate optional components
        if self.config.include_docker {
            files.extend(self.generate_docker_config(&project_name, schema)?);
        }
        
        if self.config.include_ci {
            files.extend(self.generate_ci_config(&project_name, schema)?);
        }
        
        if self.config.include_docs {
            files.extend(self.generate_documentation(&project_name, schema)?);
        }
        
        if self.config.include_examples {
            files.extend(self.generate_examples(&project_name, schema)?);
        }
        
        // Generate project manifest
        self.generate_manifest(&files)
    }
    
    /// Get project name
    fn get_project_name(&self, schema: &Schema) -> String {
        self.config.project_name.clone()
            .or_else(|| schema.name.clone())
            .unwrap_or_else(|| "linkml-project".to_string())
    }
    
    /// Generate base project structure
    fn generate_base_structure(&self, project_name: &str, schema: &Schema) -> Result<Vec<ProjectFile>, LinkMLError> {
        let mut files = Vec::new();
        
        // README.md
        files.push(ProjectFile {
            path: "README.md".to_string(),
            content: self.generate_readme(project_name, schema),
        });
        
        // LICENSE
        files.push(ProjectFile {
            path: "LICENSE".to_string(),
            content: self.generate_license(),
        });
        
        // .gitignore
        files.push(ProjectFile {
            path: ".gitignore".to_string(),
            content: self.generate_gitignore(),
        });
        
        // Schema file
        files.push(ProjectFile {
            path: format!("schema/{}.yaml", project_name),
            content: "# Original LinkML schema will be copied here\n".to_string(),
        });
        
        Ok(files)
    }
    
    /// Generate Python project files
    fn generate_python_project(&self, project_name: &str, schema: &Schema) -> Result<Vec<ProjectFile>, LinkMLError> {
        let mut files = Vec::new();
        
        // pyproject.toml
        files.push(ProjectFile {
            path: "pyproject.toml".to_string(),
            content: self.generate_python_pyproject(project_name, schema),
        });
        
        // src/__init__.py
        files.push(ProjectFile {
            path: format!("src/{}/__init__.py", project_name.replace('-', "_")),
            content: format!("\"\"\"{}.\"\"\"\n\n__version__ = \"{}\"\n", 
                schema.description.as_deref().unwrap_or("LinkML generated project"),
                self.config.version
            ),
        });
        
        // src/schema.py
        files.push(ProjectFile {
            path: format!("src/{}/schema.py", project_name.replace('-', "_")),
            content: self.generate_python_schema_module(project_name, schema),
        });
        
        // tests/__init__.py
        if self.config.include_tests {
            files.push(ProjectFile {
                path: "tests/__init__.py".to_string(),
                content: "\"\"\"Test package.\"\"\"\n".to_string(),
            });
            
            // tests/test_schema.py
            files.push(ProjectFile {
                path: "tests/test_schema.py".to_string(),
                content: self.generate_python_tests(project_name, schema),
            });
        }
        
        Ok(files)
    }
    
    /// Generate TypeScript project files
    fn generate_typescript_project(&self, project_name: &str, schema: &Schema) -> Result<Vec<ProjectFile>, LinkMLError> {
        let mut files = Vec::new();
        
        // package.json
        files.push(ProjectFile {
            path: "package.json".to_string(),
            content: self.generate_typescript_package_json(project_name, schema),
        });
        
        // tsconfig.json
        files.push(ProjectFile {
            path: "tsconfig.json".to_string(),
            content: self.generate_typescript_tsconfig(),
        });
        
        // src/index.ts
        files.push(ProjectFile {
            path: "src/index.ts".to_string(),
            content: self.generate_typescript_index(project_name, schema),
        });
        
        // src/types.ts
        files.push(ProjectFile {
            path: "src/types.ts".to_string(),
            content: "// Generated types from LinkML schema\n\nexport interface Schema {\n  // Schema types will be generated here\n}\n".to_string(),
        });
        
        // tests/schema.test.ts
        if self.config.include_tests {
            files.push(ProjectFile {
                path: "tests/schema.test.ts".to_string(),
                content: self.generate_typescript_tests(project_name, schema),
            });
        }
        
        Ok(files)
    }
    
    /// Generate Rust project files
    fn generate_rust_project(&self, project_name: &str, schema: &Schema) -> Result<Vec<ProjectFile>, LinkMLError> {
        let mut files = Vec::new();
        
        // Cargo.toml
        files.push(ProjectFile {
            path: "Cargo.toml".to_string(),
            content: self.generate_rust_cargo_toml(project_name, schema),
        });
        
        // src/lib.rs
        files.push(ProjectFile {
            path: "src/lib.rs".to_string(),
            content: self.generate_rust_lib(project_name, schema),
        });
        
        // src/schema.rs
        files.push(ProjectFile {
            path: "src/schema.rs".to_string(),
            content: "//! Generated schema types from LinkML\n\nuse serde::{Deserialize, Serialize};\n\n// Schema types will be generated here\n".to_string(),
        });
        
        // tests/integration_test.rs
        if self.config.include_tests {
            files.push(ProjectFile {
                path: "tests/integration_test.rs".to_string(),
                content: self.generate_rust_tests(project_name, schema),
            });
        }
        
        Ok(files)
    }
    
    /// Generate Java project files
    fn generate_java_project(&self, project_name: &str, schema: &Schema) -> Result<Vec<ProjectFile>, LinkMLError> {
        let mut files = Vec::new();
        
        // pom.xml
        files.push(ProjectFile {
            path: "pom.xml".to_string(),
            content: self.generate_java_pom_xml(project_name, schema),
        });
        
        // Main class
        let package_name = project_name.replace('-', "_").to_lowercase();
        files.push(ProjectFile {
            path: format!("src/main/java/com/{}/Schema.java", package_name),
            content: self.generate_java_schema_class(&package_name, schema),
        });
        
        // Test class
        if self.config.include_tests {
            files.push(ProjectFile {
                path: format!("src/test/java/com/{}/SchemaTest.java", package_name),
                content: self.generate_java_tests(&package_name, schema),
            });
        }
        
        Ok(files)
    }
    
    /// Generate Go project files
    fn generate_go_project(&self, project_name: &str, schema: &Schema) -> Result<Vec<ProjectFile>, LinkMLError> {
        let mut files = Vec::new();
        
        // go.mod
        files.push(ProjectFile {
            path: "go.mod".to_string(),
            content: self.generate_go_mod(project_name, schema),
        });
        
        // schema.go
        files.push(ProjectFile {
            path: "schema.go".to_string(),
            content: self.generate_go_schema(project_name, schema),
        });
        
        // schema_test.go
        if self.config.include_tests {
            files.push(ProjectFile {
                path: "schema_test.go".to_string(),
                content: self.generate_go_tests(project_name, schema),
            });
        }
        
        Ok(files)
    }
    
    /// Generate multi-language project files
    fn generate_multi_language_project(&self, project_name: &str, schema: &Schema) -> Result<Vec<ProjectFile>, LinkMLError> {
        let mut files = Vec::new();
        
        // Root build configuration
        files.push(ProjectFile {
            path: "Makefile".to_string(),
            content: self.generate_makefile(project_name),
        });
        
        // Language subdirectories
        files.push(ProjectFile {
            path: "python/README.md".to_string(),
            content: "# Python Implementation\n\nPython implementation of the LinkML schema.\n".to_string(),
        });
        
        files.push(ProjectFile {
            path: "typescript/README.md".to_string(),
            content: "# TypeScript Implementation\n\nTypeScript implementation of the LinkML schema.\n".to_string(),
        });
        
        files.push(ProjectFile {
            path: "rust/README.md".to_string(),
            content: "# Rust Implementation\n\nRust implementation of the LinkML schema.\n".to_string(),
        });
        
        Ok(files)
    }
    
    /// Generate Docker configuration
    fn generate_docker_config(&self, project_name: &str, _schema: &Schema) -> Result<Vec<ProjectFile>, LinkMLError> {
        let mut files = Vec::new();
        
        // Dockerfile
        let dockerfile_content = match self.config.target {
            ProjectTarget::Python => self.generate_python_dockerfile(),
            ProjectTarget::TypeScript => self.generate_node_dockerfile(),
            ProjectTarget::Rust => self.generate_rust_dockerfile(),
            ProjectTarget::Java => self.generate_java_dockerfile(),
            ProjectTarget::Go => self.generate_go_dockerfile(),
            ProjectTarget::MultiLanguage => self.generate_multi_dockerfile(),
        };
        
        files.push(ProjectFile {
            path: "Dockerfile".to_string(),
            content: dockerfile_content,
        });
        
        // docker-compose.yml
        files.push(ProjectFile {
            path: "docker-compose.yml".to_string(),
            content: self.generate_docker_compose(project_name),
        });
        
        // .dockerignore
        files.push(ProjectFile {
            path: ".dockerignore".to_string(),
            content: self.generate_dockerignore(),
        });
        
        Ok(files)
    }
    
    /// Generate CI/CD configuration
    fn generate_ci_config(&self, project_name: &str, _schema: &Schema) -> Result<Vec<ProjectFile>, LinkMLError> {
        let mut files = Vec::new();
        
        // GitHub Actions
        files.push(ProjectFile {
            path: ".github/workflows/ci.yml".to_string(),
            content: self.generate_github_actions_ci(project_name),
        });
        
        // GitLab CI
        files.push(ProjectFile {
            path: ".gitlab-ci.yml".to_string(),
            content: self.generate_gitlab_ci(project_name),
        });
        
        Ok(files)
    }
    
    /// Generate documentation
    fn generate_documentation(&self, project_name: &str, schema: &Schema) -> Result<Vec<ProjectFile>, LinkMLError> {
        let mut files = Vec::new();
        
        // docs/index.md
        files.push(ProjectFile {
            path: "docs/index.md".to_string(),
            content: format!("# {} Documentation\n\n{}\n",
                project_name,
                schema.description.as_deref().unwrap_or("LinkML schema documentation")
            ),
        });
        
        // docs/getting-started.md
        files.push(ProjectFile {
            path: "docs/getting-started.md".to_string(),
            content: self.generate_getting_started_doc(project_name),
        });
        
        // docs/api.md
        files.push(ProjectFile {
            path: "docs/api.md".to_string(),
            content: "# API Reference\n\nAPI documentation will be generated here.\n".to_string(),
        });
        
        // mkdocs.yml (for Python projects)
        if matches!(self.config.target, ProjectTarget::Python | ProjectTarget::MultiLanguage) {
            files.push(ProjectFile {
                path: "mkdocs.yml".to_string(),
                content: self.generate_mkdocs_config(project_name),
            });
        }
        
        Ok(files)
    }
    
    /// Generate examples
    fn generate_examples(&self, project_name: &str, schema: &Schema) -> Result<Vec<ProjectFile>, LinkMLError> {
        let mut files = Vec::new();
        
        match self.config.target {
            ProjectTarget::Python => {
                files.push(ProjectFile {
                    path: "examples/basic_usage.py".to_string(),
                    content: self.generate_python_example(project_name, schema),
                });
            }
            ProjectTarget::TypeScript => {
                files.push(ProjectFile {
                    path: "examples/basic-usage.ts".to_string(),
                    content: self.generate_typescript_example(project_name, schema),
                });
            }
            ProjectTarget::Rust => {
                files.push(ProjectFile {
                    path: "examples/basic_usage.rs".to_string(),
                    content: self.generate_rust_example(project_name, schema),
                });
            }
            _ => {
                files.push(ProjectFile {
                    path: "examples/README.md".to_string(),
                    content: "# Examples\n\nExample usage of the LinkML schema.\n".to_string(),
                });
            }
        }
        
        Ok(files)
    }
    
    /// Generate project manifest
    fn generate_manifest(&self, files: &[ProjectFile]) -> Result<String, LinkMLError> {
        let mut manifest = String::new();
        
        manifest.push_str("# LinkML Project Generator Manifest\n");
        manifest.push_str("# This file lists all files that will be generated\n\n");
        
        manifest.push_str("## Project Structure\n\n");
        manifest.push_str("```\n");
        
        // Build directory tree
        let mut paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
        paths.sort();
        
        for path in paths {
            let depth = path.matches('/').count();
            let indent = "  ".repeat(depth);
            let filename = path.split('/').last().unwrap_or(path);
            manifest.push_str(&format!("{}├── {}\n", indent, filename));
        }
        
        manifest.push_str("```\n\n");
        
        // File contents
        manifest.push_str("## Generated Files\n\n");
        
        for file in files {
            manifest.push_str(&format!("### {}\n\n", file.path));
            manifest.push_str("```\n");
            
            // Limit content preview to first 20 lines
            let lines: Vec<&str> = file.content.lines().take(20).collect();
            for line in lines {
                manifest.push_str(line);
                manifest.push('\n');
            }
            
            if file.content.lines().count() > 20 {
                manifest.push_str("... (truncated)\n");
            }
            
            manifest.push_str("```\n\n");
        }
        
        Ok(manifest)
    }
    
    // Helper methods for generating specific file contents
    
    fn generate_readme(&self, project_name: &str, schema: &Schema) -> String {
        format!(
            "# {}\n\n{}\n\n## Overview\n\nThis project was generated from a LinkML schema using the LinkML Project Generator.\n\n## Installation\n\n{}\n\n## Usage\n\nSee the [documentation](docs/index.md) for detailed usage instructions.\n\n## License\n\n{}\n",
            project_name,
            schema.description.as_deref().unwrap_or("A LinkML schema project"),
            self.get_installation_instructions(),
            self.get_license_name()
        )
    }
    
    fn generate_license(&self) -> String {
        match self.config.license {
            LicenseType::Mit => include_str!("../../templates/licenses/MIT").to_string(),
            LicenseType::Apache2 => include_str!("../../templates/licenses/Apache-2.0").to_string(),
            LicenseType::Bsd3 => include_str!("../../templates/licenses/BSD-3-Clause").to_string(),
            LicenseType::Gpl3 => include_str!("../../templates/licenses/GPL-3.0").to_string(),
            LicenseType::Cc0 => include_str!("../../templates/licenses/CC0-1.0").to_string(),
            LicenseType::Proprietary => "Proprietary License\n\nAll rights reserved.\n".to_string(),
        }
    }
    
    fn generate_gitignore(&self) -> String {
        let mut gitignore = String::from(
            "# General\n.DS_Store\n*.log\n.env\n.vscode/\n.idea/\n\n"
        );
        
        match self.config.target {
            ProjectTarget::Python => {
                gitignore.push_str("# Python\n__pycache__/\n*.py[cod]\n*$py.class\n*.so\n.Python\nbuild/\ndist/\n*.egg-info/\n.pytest_cache/\n.coverage\n.mypy_cache/\n.ruff_cache/\nvenv/\n");
            }
            ProjectTarget::TypeScript => {
                gitignore.push_str("# Node\nnode_modules/\ndist/\n*.tsbuildinfo\ncoverage/\n.npm/\n");
            }
            ProjectTarget::Rust => {
                gitignore.push_str("# Rust\ntarget/\nCargo.lock\n**/*.rs.bk\n");
            }
            ProjectTarget::Java => {
                gitignore.push_str("# Java\ntarget/\n*.class\n*.jar\n*.war\n");
            }
            ProjectTarget::Go => {
                gitignore.push_str("# Go\n*.exe\n*.exe~\n*.dll\n*.so\n*.dylib\n*.test\n*.out\nvendor/\n");
            }
            ProjectTarget::MultiLanguage => {
                gitignore.push_str("# Multi-language\n**/target/\n**/node_modules/\n**/dist/\n**/__pycache__/\n");
            }
        }
        
        gitignore
    }
    
    fn generate_python_pyproject(&self, project_name: &str, schema: &Schema) -> String {
        format!(
            r#"[tool.poetry]
name = "{}"
version = "{}"
description = "{}"
authors = ["{} <{}>"]
license = "{}"
readme = "README.md"

[tool.poetry.dependencies]
python = "^3.8"
linkml = "^1.3"
pydantic = "^2.0"

[tool.poetry.group.dev.dependencies]
pytest = "^7.0"
black = "^23.0"
ruff = "^0.1"
mypy = "^1.0"

[build-system]
requires = ["poetry-core"]
build-backend = "poetry.core.masonry.api"

[tool.black]
line-length = 88
target-version = ['py38']

[tool.ruff]
line-length = 88
select = ["E", "F", "I"]

[tool.mypy]
python_version = "3.8"
warn_return_any = true
warn_unused_configs = true
"#,
            project_name,
            self.config.version,
            schema.description.as_deref().unwrap_or("LinkML schema project"),
            self.config.author.as_deref().unwrap_or("Author Name"),
            self.config.author_email.as_deref().unwrap_or("author@example.com"),
            self.get_license_spdx()
        )
    }
    
    fn generate_python_schema_module(&self, project_name: &str, schema: &Schema) -> String {
        format!(
            r#""""Schema module for {}."""

from pathlib import Path
from linkml.runtime.loaders import yaml_loader
from linkml.runtime.dumpers import yaml_dumper

SCHEMA_DIR = Path(__file__).parent.parent.parent / "schema"
SCHEMA_FILE = SCHEMA_DIR / "{}.yaml"

def load_schema():
    """Load the LinkML schema."""
    return yaml_loader.load(str(SCHEMA_FILE))

def validate_data(data):
    """Validate data against the schema."""
    # Validation logic will be implemented here
    pass
"#,
            project_name,
            project_name
        )
    }
    
    fn generate_python_tests(&self, project_name: &str, _schema: &Schema) -> String {
        format!(
            r#""""Tests for {} schema."""

import pytest
from {}.schema import load_schema, validate_data

def test_load_schema():
    """Test schema loading."""
    schema = load_schema()
    assert schema is not None

def test_validate_data():
    """Test data validation."""
    # Add validation tests here
    pass
"#,
            project_name,
            project_name.replace('-', "_")
        )
    }
    
    fn generate_typescript_package_json(&self, project_name: &str, schema: &Schema) -> String {
        format!(
            r#"{{
  "name": "{}",
  "version": "{}",
  "description": "{}",
  "main": "dist/index.js",
  "types": "dist/index.d.ts",
  "scripts": {{
    "build": "tsc",
    "test": "jest",
    "lint": "eslint src --ext .ts",
    "format": "prettier --write src/**/*.ts"
  }},
  "keywords": ["linkml", "schema"],
  "author": "{} <{}>",
  "license": "{}",
  "dependencies": {{
    "ajv": "^8.0.0"
  }},
  "devDependencies": {{
    "@types/jest": "^29.0.0",
    "@types/node": "^20.0.0",
    "@typescript-eslint/eslint-plugin": "^6.0.0",
    "@typescript-eslint/parser": "^6.0.0",
    "eslint": "^8.0.0",
    "jest": "^29.0.0",
    "prettier": "^3.0.0",
    "ts-jest": "^29.0.0",
    "typescript": "^5.0.0"
  }}
}}
"#,
            project_name,
            self.config.version,
            schema.description.as_deref().unwrap_or("LinkML schema project"),
            self.config.author.as_deref().unwrap_or("Author Name"),
            self.config.author_email.as_deref().unwrap_or("author@example.com"),
            self.get_license_spdx()
        )
    }
    
    fn generate_typescript_tsconfig(&self) -> String {
        r#"{
  "compilerOptions": {
    "target": "ES2020",
    "module": "commonjs",
    "lib": ["ES2020"],
    "declaration": true,
    "outDir": "./dist",
    "rootDir": "./src",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "resolveJsonModule": true
  },
  "include": ["src/**/*"],
  "exclude": ["node_modules", "dist", "tests"]
}
"#.to_string()
    }
    
    fn generate_typescript_index(&self, project_name: &str, _schema: &Schema) -> String {
        format!(
            r#"/**
 * {} - Main entry point
 */

export * from './types';

/**
 * Load schema from file
 */
export function loadSchema(): any {{
  // Schema loading implementation
  return {{}};
}}

/**
 * Validate data against schema
 */
export function validateData(data: any): boolean {{
  // Validation implementation
  return true;
}}
"#,
            project_name
        )
    }
    
    fn generate_typescript_tests(&self, _project_name: &str, _schema: &Schema) -> String {
        r#"import { loadSchema, validateData } from '../src';

describe('Schema Tests', () => {
  test('should load schema', () => {
    const schema = loadSchema();
    expect(schema).toBeDefined();
  });

  test('should validate data', () => {
    const data = {};
    const isValid = validateData(data);
    expect(isValid).toBe(true);
  });
});
"#.to_string()
    }
    
    fn generate_rust_cargo_toml(&self, project_name: &str, schema: &Schema) -> String {
        format!(
            r#"[package]
name = "{}"
version = "{}"
edition = "2021"
authors = ["{} <{}>"]
description = "{}"
license = "{}"

[dependencies]
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
serde_yaml = "0.9"
thiserror = "1.0"

[dev-dependencies]
tokio = {{ version = "1.0", features = ["full"] }}
"#,
            project_name,
            self.config.version,
            self.config.author.as_deref().unwrap_or("Author Name"),
            self.config.author_email.as_deref().unwrap_or("author@example.com"),
            schema.description.as_deref().unwrap_or("LinkML schema project"),
            self.get_license_spdx()
        )
    }
    
    fn generate_rust_lib(&self, project_name: &str, _schema: &Schema) -> String {
        format!(
            r#"//! {} - LinkML schema implementation

pub mod schema;

use std::error::Error;

/// Load schema from file
pub fn load_schema() -> Result<(), Box<dyn Error>> {{
    // Implementation here
    Ok(())
}}

/// Validate data against schema
pub fn validate_data(data: &str) -> Result<bool, Box<dyn Error>> {{
    // Implementation here
    Ok(true)
}}
"#,
            project_name
        )
    }
    
    fn generate_rust_tests(&self, _project_name: &str, _schema: &Schema) -> String {
        r#"use linkml_project::{load_schema, validate_data};

#[test]
fn test_load_schema() {
    let result = load_schema();
    assert!(result.is_ok());
}

#[test]
fn test_validate_data() {
    let data = "{}";
    let result = validate_data(data);
    assert!(result.is_ok());
}
"#.to_string()
    }
    
    fn generate_java_pom_xml(&self, project_name: &str, schema: &Schema) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0"
         xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
         xsi:schemaLocation="http://maven.apache.org/POM/4.0.0
         http://maven.apache.org/xsd/maven-4.0.0.xsd">
    <modelVersion>4.0.0</modelVersion>

    <groupId>com.{}</groupId>
    <artifactId>{}</artifactId>
    <version>{}</version>
    <packaging>jar</packaging>

    <name>{}</name>
    <description>{}</description>

    <properties>
        <maven.compiler.source>11</maven.compiler.source>
        <maven.compiler.target>11</maven.compiler.target>
        <project.build.sourceEncoding>UTF-8</project.build.sourceEncoding>
    </properties>

    <dependencies>
        <dependency>
            <groupId>com.fasterxml.jackson.core</groupId>
            <artifactId>jackson-databind</artifactId>
            <version>2.15.0</version>
        </dependency>
        <dependency>
            <groupId>junit</groupId>
            <artifactId>junit</artifactId>
            <version>4.13.2</version>
            <scope>test</scope>
        </dependency>
    </dependencies>
</project>
"#,
            project_name.replace('-', "_").to_lowercase(),
            project_name,
            self.config.version,
            project_name,
            schema.description.as_deref().unwrap_or("LinkML schema project")
        )
    }
    
    fn generate_java_schema_class(&self, package_name: &str, _schema: &Schema) -> String {
        format!(
            r#"package com.{};

import com.fasterxml.jackson.databind.ObjectMapper;
import java.io.File;
import java.io.IOException;

/**
 * LinkML Schema implementation
 */
public class Schema {{
    
    private static final ObjectMapper mapper = new ObjectMapper();
    
    /**
     * Load schema from file
     */
    public static Object loadSchema(String filename) throws IOException {{
        return mapper.readValue(new File(filename), Object.class);
    }}
    
    /**
     * Validate data against schema
     */
    public static boolean validateData(Object data) {{
        // Validation implementation
        return true;
    }}
}}
"#,
            package_name
        )
    }
    
    fn generate_java_tests(&self, package_name: &str, _schema: &Schema) -> String {
        format!(
            r#"package com.{};

import org.junit.Test;
import static org.junit.Assert.*;

public class SchemaTest {{
    
    @Test
    public void testLoadSchema() throws Exception {{
        // Test implementation
        assertTrue(true);
    }}
    
    @Test
    public void testValidateData() {{
        Object data = new Object();
        boolean isValid = Schema.validateData(data);
        assertTrue(isValid);
    }}
}}
"#,
            package_name
        )
    }
    
    fn generate_go_mod(&self, project_name: &str, _schema: &Schema) -> String {
        format!(
            r#"module github.com/{}/{}

go 1.21

require (
    github.com/go-yaml/yaml v2.4.0
)
"#,
            self.config.author.as_deref().unwrap_or("username").to_lowercase(),
            project_name
        )
    }
    
    fn generate_go_schema(&self, project_name: &str, _schema: &Schema) -> String {
        format!(
            r#"// Package {} provides LinkML schema implementation
package {}

import (
    "encoding/json"
    "fmt"
    "os"
)

// LoadSchema loads the schema from a file
func LoadSchema(filename string) (interface{{}}, error) {{
    data, err := os.ReadFile(filename)
    if err != nil {{
        return nil, fmt.Errorf("failed to read schema: %w", err)
    }}
    
    var schema interface{{}}
    err = json.Unmarshal(data, &schema)
    if err != nil {{
        return nil, fmt.Errorf("failed to parse schema: %w", err)
    }}
    
    return schema, nil
}}

// ValidateData validates data against the schema
func ValidateData(data interface{{}}) bool {{
    // Validation implementation
    return true
}}
"#,
            project_name.replace('-', "_"),
            project_name.replace('-', "_")
        )
    }
    
    fn generate_go_tests(&self, project_name: &str, _schema: &Schema) -> String {
        format!(
            r#"package {}_test

import (
    "testing"
    . "github.com/{}/{}"
)

func TestLoadSchema(t *testing.T) {{
    // Test implementation
    t.Log("Schema loading test")
}}

func TestValidateData(t *testing.T) {{
    data := map[string]interface{{}}{{}}
    isValid := ValidateData(data)
    if !isValid {{
        t.Error("Expected data to be valid")
    }}
}}
"#,
            project_name.replace('-', "_"),
            self.config.author.as_deref().unwrap_or("username").to_lowercase(),
            project_name
        )
    }
    
    fn generate_makefile(&self, project_name: &str) -> String {
        format!(
            r#"# Makefile for {} multi-language project

.PHONY: all build test clean

all: build

build:
	@echo "Building all language implementations..."
	cd python && poetry install
	cd typescript && npm install && npm run build
	cd rust && cargo build --release

test:
	@echo "Running all tests..."
	cd python && poetry run pytest
	cd typescript && npm test
	cd rust && cargo test

clean:
	@echo "Cleaning build artifacts..."
	cd python && rm -rf dist/ build/
	cd typescript && rm -rf dist/ node_modules/
	cd rust && cargo clean
"#,
            project_name
        )
    }
    
    fn generate_python_dockerfile(&self) -> String {
        r#"FROM python:3.11-slim

WORKDIR /app

COPY pyproject.toml poetry.lock* ./
RUN pip install poetry && \
    poetry config virtualenvs.create false && \
    poetry install --no-interaction --no-ansi

COPY . .

CMD ["python", "-m", "linkml_project"]
"#.to_string()
    }
    
    fn generate_node_dockerfile(&self) -> String {
        r#"FROM node:20-alpine

WORKDIR /app

COPY package*.json ./
RUN npm ci --only=production

COPY . .
RUN npm run build

CMD ["node", "dist/index.js"]
"#.to_string()
    }
    
    fn generate_rust_dockerfile(&self) -> String {
        r#"FROM rust:1.75 as builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/linkml-project /usr/local/bin/

CMD ["linkml-project"]
"#.to_string()
    }
    
    fn generate_java_dockerfile(&self) -> String {
        r#"FROM maven:3.8-openjdk-11 as builder

WORKDIR /app
COPY pom.xml ./
RUN mvn dependency:go-offline

COPY src ./src
RUN mvn package

FROM openjdk:11-jre-slim
COPY --from=builder /app/target/*.jar app.jar

CMD ["java", "-jar", "app.jar"]
"#.to_string()
    }
    
    fn generate_go_dockerfile(&self) -> String {
        r#"FROM golang:1.21-alpine as builder

WORKDIR /app
COPY go.mod go.sum ./
RUN go mod download

COPY . .
RUN go build -o linkml-project

FROM alpine:latest
RUN apk --no-cache add ca-certificates
COPY --from=builder /app/linkml-project /usr/local/bin/

CMD ["linkml-project"]
"#.to_string()
    }
    
    fn generate_multi_dockerfile(&self) -> String {
        r#"FROM ubuntu:22.04

RUN apt-get update && apt-get install -y \
    python3 python3-pip \
    nodejs npm \
    cargo \
    openjdk-11-jdk \
    golang-go \
    make

WORKDIR /app
COPY . .

RUN make build

CMD ["make", "test"]
"#.to_string()
    }
    
    fn generate_docker_compose(&self, project_name: &str) -> String {
        format!(
            r#"version: '3.8'

services:
  app:
    build: .
    container_name: {}-app
    volumes:
      - ./schema:/app/schema
    environment:
      - LOG_LEVEL=info
    ports:
      - "8080:8080"
"#,
            project_name
        )
    }
    
    fn generate_dockerignore(&self) -> String {
        r#"# Git
.git/
.gitignore

# CI
.github/
.gitlab-ci.yml

# Documentation
docs/
*.md

# Testing
tests/
coverage/

# Development
.vscode/
.idea/
*.swp
*.swo

# Language specific
__pycache__/
node_modules/
target/
*.pyc
"#.to_string()
    }
    
    fn generate_github_actions_ci(&self, project_name: &str) -> String {
        let test_step = match self.config.target {
            ProjectTarget::Python => r#"      - name: Test
        run: |
          poetry install
          poetry run pytest"#,
            ProjectTarget::TypeScript => r#"      - name: Test
        run: |
          npm ci
          npm test"#,
            ProjectTarget::Rust => r#"      - name: Test
        run: cargo test"#,
            ProjectTarget::Java => r#"      - name: Test
        run: mvn test"#,
            ProjectTarget::Go => r#"      - name: Test
        run: go test ./..."#,
            ProjectTarget::MultiLanguage => r#"      - name: Test
        run: make test"#,
        };
        
        format!(
            r#"name: CI

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

jobs:
  test:
    name: Test
    runs-on: ubuntu-latest
    
    steps:
      - uses: actions/checkout@v4
      
      - name: Setup
        uses: actions/setup-node@v4
        with:
          node-version: '20'
      
{}

      - name: Lint
        run: echo "Linting..."
"#,
            test_step
        )
    }
    
    fn generate_gitlab_ci(&self, _project_name: &str) -> String {
        let test_script = match self.config.target {
            ProjectTarget::Python => "poetry install && poetry run pytest",
            ProjectTarget::TypeScript => "npm ci && npm test",
            ProjectTarget::Rust => "cargo test",
            ProjectTarget::Java => "mvn test",
            ProjectTarget::Go => "go test ./...",
            ProjectTarget::MultiLanguage => "make test",
        };
        
        format!(
            r#"stages:
  - test
  - build
  - deploy

test:
  stage: test
  script:
    - {}

build:
  stage: build
  script:
    - echo "Building..."

deploy:
  stage: deploy
  script:
    - echo "Deploying..."
  only:
    - main
"#,
            test_script
        )
    }
    
    fn generate_getting_started_doc(&self, project_name: &str) -> String {
        format!(
            r#"# Getting Started with {}

## Prerequisites

{}

## Installation

{}

## Quick Start

1. Clone the repository:
   ```bash
   git clone https://github.com/yourusername/{}.git
   cd {}
   ```

2. Install dependencies:
   {}

3. Run tests:
   {}

## Next Steps

- Read the [API documentation](api.md)
- Check out the [examples](../examples/)
- Contribute to the project
"#,
            project_name,
            self.get_prerequisites(),
            self.get_installation_instructions(),
            project_name,
            project_name,
            self.get_install_command(),
            self.get_test_command()
        )
    }
    
    fn generate_mkdocs_config(&self, project_name: &str) -> String {
        format!(
            r#"site_name: {} Documentation
site_description: Documentation for the {} LinkML project
site_author: {}

theme:
  name: material
  features:
    - navigation.sections
    - navigation.expand
    - search.highlight

nav:
  - Home: index.md
  - Getting Started: getting-started.md
  - API Reference: api.md
  - Examples: examples/

plugins:
  - search
  - mkdocstrings:
      handlers:
        python:
          paths: [src]

markdown_extensions:
  - pymdownx.highlight
  - pymdownx.superfences
  - admonition
"#,
            project_name,
            project_name,
            self.config.author.as_deref().unwrap_or("Author")
        )
    }
    
    fn generate_python_example(&self, project_name: &str, _schema: &Schema) -> String {
        format!(
            r#"#!/usr/bin/env python3
"""Basic usage example for {}."""

from {}.schema import load_schema, validate_data

def main():
    # Load the schema
    schema = load_schema()
    print(f"Loaded schema: {{schema}}")
    
    # Example data
    data = {{
        "id": "example-001",
        "name": "Example Entity"
    }}
    
    # Validate data
    is_valid = validate_data(data)
    print(f"Data is valid: {{is_valid}}")

if __name__ == "__main__":
    main()
"#,
            project_name,
            project_name.replace('-', "_")
        )
    }
    
    fn generate_typescript_example(&self, project_name: &str, _schema: &Schema) -> String {
        format!(
            r#"/**
 * Basic usage example for {}
 */

import {{ loadSchema, validateData }} from '../src';

async function main() {{
    // Load the schema
    const schema = loadSchema();
    console.log('Loaded schema:', schema);
    
    // Example data
    const data = {{
        id: 'example-001',
        name: 'Example Entity'
    }};
    
    // Validate data
    const isValid = validateData(data);
    console.log('Data is valid:', isValid);
}}

main().catch(console.error);
"#,
            project_name
        )
    }
    
    fn generate_rust_example(&self, _project_name: &str, _schema: &Schema) -> String {
        r#"//! Basic usage example

use linkml_project::{load_schema, validate_data};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Load the schema
    let schema = load_schema()?;
    println!("Loaded schema successfully");
    
    // Example data
    let data = r#"
        {
            "id": "example-001",
            "name": "Example Entity"
        }
    "#;
    
    // Validate data
    let is_valid = validate_data(data)?;
    println!("Data is valid: {}", is_valid);
    
    Ok(())
}
"#.to_string()
    }
    
    // Helper methods
    
    fn get_installation_instructions(&self) -> &str {
        match self.config.target {
            ProjectTarget::Python => "```bash\npip install poetry\npoetry install\n```",
            ProjectTarget::TypeScript => "```bash\nnpm install\n```",
            ProjectTarget::Rust => "```bash\ncargo build\n```",
            ProjectTarget::Java => "```bash\nmvn install\n```",
            ProjectTarget::Go => "```bash\ngo mod download\n```",
            ProjectTarget::MultiLanguage => "```bash\nmake build\n```",
        }
    }
    
    fn get_prerequisites(&self) -> &str {
        match self.config.target {
            ProjectTarget::Python => "- Python 3.8 or higher\n- Poetry package manager",
            ProjectTarget::TypeScript => "- Node.js 16 or higher\n- npm or yarn",
            ProjectTarget::Rust => "- Rust 1.70 or higher\n- Cargo",
            ProjectTarget::Java => "- Java 11 or higher\n- Maven 3.6 or higher",
            ProjectTarget::Go => "- Go 1.19 or higher",
            ProjectTarget::MultiLanguage => "- Make\n- Python 3.8+\n- Node.js 16+\n- Rust 1.70+\n- Java 11+\n- Go 1.19+",
        }
    }
    
    fn get_install_command(&self) -> &str {
        match self.config.target {
            ProjectTarget::Python => "```bash\n   poetry install\n   ```",
            ProjectTarget::TypeScript => "```bash\n   npm install\n   ```",
            ProjectTarget::Rust => "```bash\n   cargo build\n   ```",
            ProjectTarget::Java => "```bash\n   mvn install\n   ```",
            ProjectTarget::Go => "```bash\n   go mod download\n   ```",
            ProjectTarget::MultiLanguage => "```bash\n   make build\n   ```",
        }
    }
    
    fn get_test_command(&self) -> &str {
        match self.config.target {
            ProjectTarget::Python => "```bash\n   poetry run pytest\n   ```",
            ProjectTarget::TypeScript => "```bash\n   npm test\n   ```",
            ProjectTarget::Rust => "```bash\n   cargo test\n   ```",
            ProjectTarget::Java => "```bash\n   mvn test\n   ```",
            ProjectTarget::Go => "```bash\n   go test ./...\n   ```",
            ProjectTarget::MultiLanguage => "```bash\n   make test\n   ```",
        }
    }
    
    fn get_license_name(&self) -> &str {
        match self.config.license {
            LicenseType::Mit => "MIT License",
            LicenseType::Apache2 => "Apache License 2.0",
            LicenseType::Bsd3 => "BSD 3-Clause License",
            LicenseType::Gpl3 => "GNU General Public License v3.0",
            LicenseType::Cc0 => "Creative Commons Zero v1.0 Universal",
            LicenseType::Proprietary => "Proprietary License",
        }
    }
    
    fn get_license_spdx(&self) -> &str {
        match self.config.license {
            LicenseType::Mit => "MIT",
            LicenseType::Apache2 => "Apache-2.0",
            LicenseType::Bsd3 => "BSD-3-Clause",
            LicenseType::Gpl3 => "GPL-3.0",
            LicenseType::Cc0 => "CC0-1.0",
            LicenseType::Proprietary => "Proprietary",
        }
    }
}

impl Generator for ProjectGenerator {
    fn generate(&self, schema: &Schema) -> Result<String, LinkMLError> {
        self.generate_project(schema)
    }
    
    fn get_file_extension(&self) -> &str {
        "txt" // Manifest is a text file
    }
    
    fn get_default_filename(&self) -> &str {
        "project_manifest"
    }
}

// License templates (simplified versions)
mod templates {
    pub mod licenses {
        pub const MIT: &str = r#"MIT License

Copyright (c) [year] [fullname]

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
"#;

        pub const APACHE_2_0: &str = r#"Apache License
Version 2.0, January 2004

Copyright [yyyy] [name of copyright owner]

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
"#;

        pub const BSD_3_CLAUSE: &str = r#"BSD 3-Clause License

Copyright (c) [year], [fullname]
All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

1. Redistributions of source code must retain the above copyright notice, this
   list of conditions and the following disclaimer.

2. Redistributions in binary form must reproduce the above copyright notice,
   this list of conditions and the following disclaimer in the documentation
   and/or other materials provided with the distribution.

3. Neither the name of the copyright holder nor the names of its
   contributors may be used to endorse or promote products derived from
   this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
"#;

        pub const GPL_3_0: &str = r#"GNU GENERAL PUBLIC LICENSE
Version 3, 29 June 2007

Copyright (C) [year] [fullname]

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
"#;

        pub const CC0_1_0: &str = r#"CC0 1.0 Universal

CREATIVE COMMONS CORPORATION IS NOT A LAW FIRM AND DOES NOT PROVIDE LEGAL SERVICES.

Statement of Purpose

The laws of most jurisdictions throughout the world automatically confer exclusive
Copyright and Related Rights (defined below) upon the creator and subsequent owner(s)
(each and all, an "owner") of an original work of authorship and/or a database
(each, a "Work").

To the greatest extent permitted by, but not in contravention of, applicable law,
Affirmer hereby overtly, fully, permanently, irrevocably and unconditionally waives,
abandons, and surrenders all of Affirmer's Copyright and Related Rights and associated
claims and causes of action, whether now known or unknown (including existing as well
as future claims and causes of action), in the Work.
"#;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::schema::SchemaDefinition;
    
    #[test]
    fn test_project_generation() {
        let mut schema = SchemaDefinition::default();
        schema.name = Some("TestProject".to_string());
        schema.description = Some("A test project".to_string());
        
        let config = ProjectGeneratorConfig {
            project_name: Some("test-project".to_string()),
            target: ProjectTarget::Python,
            include_docker: false,
            include_ci: false,
            ..Default::default()
        };
        
        let generator = ProjectGenerator::new(config);
        let result = generator.generate(&Schema(schema)).expect("should generate project");
        
        assert!(result.contains("# LinkML Project Generator Manifest"));
        assert!(result.contains("README.md"));
        assert!(result.contains("pyproject.toml"));
    }
}