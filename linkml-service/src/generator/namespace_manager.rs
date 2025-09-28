//! Namespace manager generator for `LinkML` schemas
//!
//! This module generates namespace management utilities from `LinkML` schemas,
//! including prefix expansion/contraction, URI validation, and namespace resolution.

use crate::generator::traits::{Generator, GeneratorConfig};
use linkml_core::error::LinkMLError;
use linkml_core::types::{PrefixDefinition, SchemaDefinition};
use std::fmt::Write;

/// Namespace manager generator configuration
#[derive(Debug, Clone)]
pub struct NamespaceManagerGeneratorConfig {
    /// Base generator configuration
    pub base: GeneratorConfig,
    /// Target language for generation
    pub target_language: TargetLanguage,
    /// Whether to include validation methods
    pub include_validation: bool,
    /// Whether to include utility methods
    pub include_utilities: bool,
    /// Whether to generate thread-safe implementation
    pub thread_safe: bool,
    /// Class name for the generated manager
    pub class_name: String,
}

/// Supported target languages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetLanguage {
    /// Python namespace manager
    Python,
    /// JavaScript/TypeScript namespace manager
    JavaScript,
    /// Rust namespace manager
    Rust,
    /// Java namespace manager
    Java,
    /// Go namespace manager
    Go,
}

impl Default for NamespaceManagerGeneratorConfig {
    fn default() -> Self {
        Self {
            base: GeneratorConfig::default(),
            target_language: TargetLanguage::Python,
            include_validation: true,
            include_utilities: true,
            thread_safe: false,
            class_name: "NamespaceManager".to_string(),
        }
    }
}

/// Namespace manager generator
pub struct NamespaceManagerGenerator {
    config: NamespaceManagerGeneratorConfig,
    /// Additional generator options for customization
    options: super::traits::GeneratorOptions,
}

impl NamespaceManagerGenerator {
    /// Create a new namespace manager generator
    #[must_use]
    pub fn new(config: NamespaceManagerGeneratorConfig) -> Self {
        Self {
            config,
            options: super::traits::GeneratorOptions::default(),
        }
    }

    /// Create generator with options
    #[must_use]
    pub fn with_options(options: super::traits::GeneratorOptions) -> Self {
        let config = NamespaceManagerGeneratorConfig::default();
        Self { config, options }
    }

    /// Get prefix reference from `PrefixDefinition`
    fn get_prefix_reference(prefix_def: &PrefixDefinition) -> &str {
        match prefix_def {
            PrefixDefinition::Simple(url) => url,
            PrefixDefinition::Complex {
                prefix_reference, ..
            } => prefix_reference.as_deref().unwrap_or(""),
        }
    }

    /// Get indentation string based on options
    fn get_indent(&self) -> String {
        match &self.options.indent {
            super::traits::IndentStyle::Spaces(n) => " ".repeat(*n),
            super::traits::IndentStyle::Tabs => "\t".to_string(),
        }
    }

    /// Get custom option value
    fn get_custom_option(&self, key: &str) -> Option<&String> {
        self.options.custom.get(key)
    }

    /// Generate namespace manager for the configured language
    fn generate_manager(&self, schema: &SchemaDefinition) -> String {
        match self.config.target_language {
            TargetLanguage::Python => self.generate_python(schema),
            TargetLanguage::JavaScript => self.generate_javascript(schema),
            TargetLanguage::Rust => self.generate_rust(schema),
            TargetLanguage::Java => self.generate_java(schema),
            TargetLanguage::Go => self.generate_go(schema),
        }
    }

    /// Generate Python namespace manager
    fn generate_python(&self, schema: &SchemaDefinition) -> String {
        let mut output = String::new();

        output.push_str(&self.generate_python_header());
        output.push_str(&self.generate_python_imports());

        output.push_str(&self.generate_python_class_definition(schema));

        output.push_str(&self.generate_python_methods());
        output.push_str(&self.generate_python_properties());

        // Generate tests if option is enabled
        if self.options.generate_tests {
            output.push_str(&self.generate_python_tests(schema));
        }

        output
    }

    /// Generate Python header section
    fn generate_python_header(&self) -> String {
        let mut output = String::new();

        // Header
        output.push_str("#!/usr/bin/env python3\n");

        // Include documentation if option is enabled
        if self.options.include_docs {
            output.push_str("\"\"\"Namespace manager generated from LinkML schema");

            // Add custom author/license info if provided
            if let Some(author) = self.get_custom_option("author") {
                use std::fmt::Write;
                write!(output, "\n\nAuthor: {author}").expect("Writing to string cannot fail");
            }
            if let Some(license) = self.get_custom_option("license") {
                use std::fmt::Write;
                write!(output, "\nLicense: {license}").expect("Writing to string cannot fail");
            }

            output.push_str("\"\"\"\n\n");
        }

        output
    }

    /// Generate Python imports section
    fn generate_python_imports(&self) -> String {
        let mut output = String::new();

        // Imports
        output.push_str("from typing import Dict, Optional, List, Tuple, Set\n");
        output.push_str("import re\n");
        if self.config.thread_safe {
            output.push_str("import threading\n");
        }
        output.push_str("\n\n");

        output
    }

    /// Generate Python class definition and constructor
    fn generate_python_class_definition(&self, schema: &SchemaDefinition) -> String {
        let mut output = String::new();

        // Class definition
        writeln!(output, "class {}:", self.config.class_name)
            .expect("writeln! to String should never fail");
        output.push_str(
            "    \"\"\"Manages namespace prefixes and URI expansion/contraction\"\"\"\n\n",
        );

        // Constructor
        output.push_str("    def __init__(self):\n");
        output.push_str(
            "        \"\"\"Initialize namespace manager with predefined prefixes\"\"\"\n",
        );
        if self.config.thread_safe {
            output.push_str("        self._lock = threading.RLock()\n");
        }

        // Initialize prefix mappings
        output.push_str("        self.prefixes: Dict[str, str] = {\n");
        if !schema.prefixes.is_empty() {
            use std::fmt::Write;
            for (prefix, expansion) in &schema.prefixes {
                let _ = writeln!(
                    output,
                    "            '{}': '{}',",
                    prefix,
                    Self::get_prefix_reference(expansion)
                );
            }
        }
        // Add common prefixes
        output.push_str("            # Common semantic web prefixes\n");
        output.push_str("            'rdf': 'http://www.w3.org/1999/02/22-rdf-syntax-ns#',\n");
        output.push_str("            'rdfs': 'http://www.w3.org/2000/01/rdf-schema#',\n");
        output.push_str("            'xsd': 'http://www.w3.org/2001/XMLSchema#',\n");
        output.push_str("            'owl': 'http://www.w3.org/2002/07/owl#',\n");
        output.push_str("        }\n");

        // Reverse mapping for contraction
        output.push_str("        self.namespaces: Dict[str, str] = {\n");
        output.push_str("            v: k for k, v in self._prefixes.items()\n");
        output.push_str("        }\n");

        // Default prefix
        if let Some(default_prefix) = &schema.default_prefix {
            writeln!(output, "        self._default_prefix = '{default_prefix}'")
                .expect("LinkML operation should succeed");
        } else {
            output.push_str("        self._default_prefix = None\n");
        }
        output.push('\n');

        output
    }

    /// Generate Python core methods
    fn generate_python_methods(&self) -> String {
        let mut output = String::new();

        // Core methods
        output.push_str(&self.generate_python_expand_method());
        output.push_str(&self.generate_python_contract_method());
        output.push_str(&self.generate_python_bind_method());

        if self.config.include_validation {
            output.push_str(&Self::generate_python_validation_methods());
        }

        if self.config.include_utilities {
            output.push_str(&self.generate_python_utility_methods());
        }

        output
    }

    /// Generate Python property methods
    fn generate_python_properties(&self) -> String {
        let mut output = String::new();

        // Property methods
        output.push_str("    @property\n");
        output.push_str("    def prefixes(self) -> Dict[str, str]:\n");
        output.push_str("        \"\"\"Get copy of prefix mappings\"\"\"\n");
        if self.config.thread_safe {
            output.push_str("        with self._lock:\n");
            output.push_str("            return self._prefixes.copy()\n");
        } else {
            output.push_str("        return self._prefixes.copy()\n");
        }
        output.push('\n');

        output.push_str("    @property\n");
        output.push_str("    def namespaces(self) -> Dict[str, str]:\n");
        output.push_str("        \"\"\"Get copy of namespace mappings\"\"\"\n");
        if self.config.thread_safe {
            output.push_str("        with self._lock:\n");
            output.push_str("            return self._namespaces.copy()\n");
        } else {
            output.push_str("        return self._namespaces.copy()\n");
        }
        output.push('\n');

        output
    }

    /// Generate Python expand method
    fn generate_python_expand_method(&self) -> String {
        let mut method = String::new();

        method.push_str(
            "    def expand(self, curie: str) -> str:
",
        );
        method.push_str(
            "        \"\"\"Expand a CURIE to a full URI
",
        );
        method.push_str(
            "        
",
        );
        method.push_str(
            "        Args:
",
        );
        method.push_str(
            "            curie: Compact URI (e.g., 'ex:Person')
",
        );
        method.push_str(
            "            
",
        );
        method.push_str(
            "        Returns:
",
        );
        method.push_str(
            "            Expanded URI
",
        );
        method.push_str(
            "            
",
        );
        method.push_str(
            "        Raises:
",
        );
        method.push_str(
            "            ValueError: If prefix is not registered
",
        );
        method.push_str(
            "        \"\"\"
",
        );

        if self.config.thread_safe {
            method.push_str(
                "        with self._lock:
",
            );
            method.push_str("            ");
        }

        method.push_str(
            "        if ':' not in curie:
",
        );
        method.push_str(
            "            # Use default prefix if available
",
        );
        method.push_str(
            "            if self._default_prefix and self._default_prefix in self._prefixes:
",
        );
        method.push_str(
            "                return self._prefixes[self._default_prefix] + curie
",
        );
        method.push_str(
            "            return curie
",
        );
        method.push_str(
            "        
",
        );
        method.push_str(
            "        prefix, local_name = curie.split(':', 1)
",
        );
        method.push_str(
            "        
",
        );
        method.push_str(
            "        if prefix in self._prefixes:
",
        );
        method.push_str(
            "            return self._prefixes[prefix] + local_name
",
        );
        method.push_str(
            "        
",
        );
        method.push_str(
            "        raise ValueError(f\"Unknown prefix: {prefix}\")
",
        );
        method.push('\n');

        method
    }

    /// Generate Python contract method
    fn generate_python_contract_method(&self) -> String {
        let mut method = String::new();

        method.push_str(
            "    def contract(self, uri: str) -> Optional[str]:
",
        );
        method.push_str(
            "        \"\"\"Contract a URI to a CURIE if possible
",
        );
        method.push_str(
            "        
",
        );
        method.push_str(
            "        Args:
",
        );
        method.push_str(
            "            uri: Full URI to contract
",
        );
        method.push_str(
            "            
",
        );
        method.push_str(
            "        Returns:
",
        );
        method.push_str(
            "            CURIE if contraction possible, otherwise None
",
        );
        method.push_str(
            "        \"\"\"
",
        );

        if self.config.thread_safe {
            method.push_str(
                "        with self._lock:
",
            );
            method.push_str("            ");
        }

        method.push_str(
            "        # Find the longest matching namespace
",
        );
        method.push_str(
            "        best_match = None
",
        );
        method.push_str(
            "        best_length = 0
",
        );
        method.push_str(
            "        
",
        );
        method.push_str(
            "        for namespace, prefix in self._namespaces.items():
",
        );
        method.push_str(
            "            if uri.startswith(namespace) and len(namespace) > best_length:
",
        );
        method.push_str(
            "                best_match = (namespace, prefix)
",
        );
        method.push_str(
            "                best_length = len(namespace)
",
        );
        method.push_str(
            "        
",
        );
        method.push_str(
            "        if best_match:
",
        );
        method.push_str(
            "            namespace, prefix = best_match
",
        );
        method.push_str(
            "            local_name = uri[len(namespace):]
",
        );
        method.push_str(
            "            return f\"{prefix}:{local_name}\"
",
        );
        method.push_str(
            "        
",
        );
        method.push_str(
            "        return None
",
        );
        method.push('\n');

        method
    }

    /// Generate Python bind method
    fn generate_python_bind_method(&self) -> String {
        let mut method = String::new();

        method.push_str(
            "    def bind(self, prefix: str, namespace: str) -> None:
",
        );
        method.push_str(
            "        \"\"\"Bind a new prefix to a namespace
",
        );
        method.push_str(
            "        
",
        );
        method.push_str(
            "        Args:
",
        );
        method.push_str(
            "            prefix: Prefix to bind
",
        );
        method.push_str(
            "            namespace: Namespace URI
",
        );
        method.push_str(
            "        \"\"\"
",
        );

        if self.config.thread_safe {
            method.push_str(
                "        with self._lock:
",
            );
            method.push_str("            ");
        }

        method.push_str(
            "        self._prefixes[prefix] = namespace
",
        );
        method.push_str(
            "        self._namespaces[namespace] = prefix
",
        );
        method.push('\n');

        method
    }

    /// Generate Python validation methods
    fn generate_python_validation_methods() -> String {
        let mut methods = String::new();

        // Validate URI
        methods.push_str(
            "    def is_valid_uri(self, uri: str) -> bool:
",
        );
        methods.push_str(
            "        \"\"\"Check if a string is a valid URI\"\"\"
",
        );
        methods.push_str(
            "        uri_pattern = re.compile(
",
        );
        methods.push_str(
            "            r'^[a-zA-Z][a-zA-Z0-9+.-]*:'
",
        );
        methods.push_str(
            "            r'(?://(?:[a-zA-Z0-9._~-]|%[0-9A-Fa-f]{2})*'
",
        );
        methods.push_str(
            "            r'(?::[0-9]*)?'
",
        );
        methods.push_str(
            "            r'(?:/(?:[a-zA-Z0-9._~-]|%[0-9A-Fa-f]{2})*)*)?'
",
        );
        methods.push_str(
            "        )
",
        );
        methods.push_str(
            "        return bool(uri_pattern.match(uri))
",
        );
        methods.push('\n');

        // Validate CURIE
        methods.push_str(
            "    def is_valid_curie(self, curie: str) -> bool:
",
        );
        methods.push_str(
            "        \"\"\"Check if a string is a valid CURIE\"\"\"
",
        );
        methods.push_str(
            "        if ':' not in curie:
",
        );
        methods.push_str(
            "            return False
",
        );
        methods.push_str(
            "        prefix, local = curie.split(':', 1)
",
        );
        methods.push_str(
            "        return prefix in self._prefixes
",
        );
        methods.push('\n');

        // Validate prefix
        methods.push_str(
            "    def is_valid_prefix(self, prefix: str) -> bool:
",
        );
        methods.push_str(
            "        \"\"\"Check if a prefix is valid\"\"\"
",
        );
        methods.push_str(
            "        return bool(re.match(r'^[a-zA-Z_][a-zA-Z0-9_-]*$', prefix))
",
        );
        methods.push('\n');

        methods
    }

    /// Generate Python utility methods
    fn generate_python_utility_methods(&self) -> String {
        let mut methods = String::new();

        // Get all CURIEs for a namespace
        methods.push_str(
            "    def get_curies_for_namespace(self, namespace: str) -> List[str]:
",
        );
        methods.push_str(
            "        \"\"\"Get all registered CURIEs for a namespace\"\"\"
",
        );
        if self.config.thread_safe {
            methods.push_str(
                "        with self._lock:
",
            );
            methods.push_str("            ");
        }
        methods.push_str(
            "        return [f\"{p}:\" for p, ns in self._prefixes.items() if ns == namespace]
",
        );
        methods.push('\n');

        // Normalize URI
        methods.push_str(
            "    def normalize(self, uri_or_curie: str) -> str:
",
        );
        methods.push_str(
            "        \"\"\"Normalize a URI or CURIE to full URI form\"\"\"
",
        );
        methods.push_str(
            "        if self.is_valid_curie(uri_or_curie):
",
        );
        methods.push_str(
            "            return self.expand(uri_or_curie)
",
        );
        methods.push_str(
            "        return uri_or_curie
",
        );
        methods.push('\n');

        // Export to different formats
        methods.push_str(
            "    def export_turtle(self) -> str:
",
        );
        methods.push_str(
            "        \"\"\"Export prefixes in Turtle format\"\"\"
",
        );
        methods.push_str(
            "        lines = []
",
        );
        if self.config.thread_safe {
            methods.push_str(
                "        with self._lock:
",
            );
            methods.push_str("            ");
        }
        methods.push_str(
            "        for prefix, namespace in sorted(self._prefixes.items()):
",
        );
        methods.push_str(
            "            lines.append(f\"@prefix {prefix}: <{namespace}> .\")
",
        );
        methods.push_str(
            "        return '\
'.join(lines)
",
        );
        methods.push('\n');

        methods.push_str(
            "    def export_sparql(self) -> str:
",
        );
        methods.push_str(
            "        \"\"\"Export prefixes in SPARQL format\"\"\"
",
        );
        methods.push_str(
            "        lines = []
",
        );
        if self.config.thread_safe {
            methods.push_str(
                "        with self._lock:
",
            );
            methods.push_str("            ");
        }
        methods.push_str(
            "        for prefix, namespace in sorted(self._prefixes.items()):
",
        );
        methods.push_str(
            "            lines.append(f\"PREFIX {prefix}: <{namespace}>\")
",
        );
        methods.push_str(
            "        return '\
'.join(lines)
",
        );
        methods.push('\n');

        methods
    }

    /// Generate Python tests for the namespace manager
    fn generate_python_tests(&self, schema: &SchemaDefinition) -> String {
        use std::fmt::Write;

        let mut tests = String::new();

        if self.options.include_docs {
            tests.push_str(
                "
# Test suite for namespace manager
",
            );
        }

        tests.push_str(
            "import unittest

",
        );
        writeln!(
            tests,
            "class Test{}(unittest.TestCase):",
            self.config.class_name
        )
        .expect("Writing to string cannot fail");

        let indent = self.get_indent();

        // Test basic functionality
        writeln!(tests, "{indent}def setUp(self):").expect("Writing to string cannot fail");
        writeln!(
            tests,
            "{}{}self.manager = {}()",
            indent, indent, self.config.class_name
        )
        .expect("Writing to string cannot fail");
        tests.push('\n');

        writeln!(tests, "{indent}def test_expand_known_prefix(self):")
            .expect("Writing to string cannot fail");
        writeln!(
            tests,
            "{indent}{indent}\"\"\"Test expanding a known prefix\"\"\""
        )
        .expect("Writing to string cannot fail");

        // Use schema prefixes for realistic tests
        if let Some((prefix, prefix_def)) = schema.prefixes.iter().next() {
            let namespace = match prefix_def {
                PrefixDefinition::Simple(url) => url.as_str(),
                PrefixDefinition::Complex {
                    prefix_reference, ..
                } => prefix_reference.as_deref().unwrap_or("http://example.org/"),
            };
            writeln!(
                tests,
                "{indent}{indent}result = self.manager.expand('{prefix}:test')"
            )
            .expect("Writing to string cannot fail");
            writeln!(
                tests,
                "{indent}{indent}self.assertEqual(result, '{namespace}test')"
            )
            .expect("Writing to string cannot fail");
        } else {
            writeln!(tests, "{indent}{indent}# No prefixes defined in schema")
                .expect("Writing to string cannot fail");
            writeln!(tests, "{indent}{indent}pass").expect("Writing to string cannot fail");
        }
        tests.push('\n');

        writeln!(tests, "{indent}def test_contract_known_uri(self):")
            .expect("Writing to string cannot fail");
        writeln!(
            tests,
            "{indent}{indent}\"\"\"Test contracting a known URI\"\"\""
        )
        .expect("Writing to string cannot fail");
        writeln!(
            tests,
            "{indent}{indent}# Add test implementation based on schema"
        )
        .expect("Writing to string cannot fail");
        writeln!(tests, "{indent}{indent}pass").expect("Writing to string cannot fail");
        tests.push('\n');

        tests.push_str(
            "if __name__ == '__main__':
",
        );
        writeln!(tests, "{indent}unittest.main()").expect("Writing to string cannot fail");
        tests.push('\n');

        tests
    }

    /// Generate JavaScript namespace manager
    fn generate_javascript(&self, schema: &SchemaDefinition) -> String {
        let mut output = String::new();

        output.push_str(&Self::generate_javascript_header());
        output.push_str(&self.generate_javascript_class_definition(schema));
        output.push_str(&self.generate_javascript_methods());
        output.push_str(&self.generate_javascript_footer());

        output
    }

    /// Generate JavaScript header
    fn generate_javascript_header() -> String {
        let mut output = String::new();

        output.push_str("/**\n");
        output.push_str(" * Namespace manager generated from LinkML schema\n");
        output.push_str(" */\n\n");

        output
    }

    /// Generate JavaScript class definition
    fn generate_javascript_class_definition(&self, schema: &SchemaDefinition) -> String {
        use std::fmt::Write;
        let mut output = String::new();

        // Class definition
        writeln!(output, "class {} {{", self.config.class_name)
            .expect("writeln! to String should never fail");

        // Constructor
        output.push_str("  constructor() {\n");
        output.push_str("    /**\n");
        output.push_str("     * @type {Map<string, string>}\n");
        output.push_str("     * @private\n");
        output.push_str("     */\n");
        output.push_str("    this._prefixes = new Map([\n");

        // Add schema prefixes
        if !schema.prefixes.is_empty() {
            use std::fmt::Write;
            for (prefix, expansion) in &schema.prefixes {
                let _ = writeln!(
                    output,
                    "      ['{}', '{}'],",
                    prefix,
                    Self::get_prefix_reference(expansion)
                );
            }
        }

        // Add common prefixes
        output.push_str("      // Common semantic web prefixes\n");
        output.push_str("      ['rdf', 'http://www.w3.org/1999/02/22-rdf-syntax-ns#'],\n");
        output.push_str("      ['rdfs', 'http://www.w3.org/2000/01/rdf-schema#'],\n");
        output.push_str("      ['xsd', 'http://www.w3.org/2001/XMLSchema#'],\n");
        output.push_str("      ['owl', 'http://www.w3.org/2002/07/owl#'],\n");
        output.push_str("    ]);\n\n");

        output.push_str("    /**\n");
        output.push_str("     * @type {Map<string, string>}\n");
        output.push_str("     * @private\n");
        output.push_str("     */\n");
        output.push_str("    this._namespaces = new Map();\n");
        output.push_str("    for (const [prefix, namespace] of this._prefixes) {\n");
        output.push_str("      this._namespaces.set(namespace, prefix);\n");
        output.push_str("    }\n\n");

        // Default prefix
        if let Some(default_prefix) = &schema.default_prefix {
            writeln!(output, "    this._defaultPrefix = '{default_prefix}';")
                .expect("LinkML operation should succeed");
        } else {
            output.push_str("    this._defaultPrefix = null;\n");
        }
        output.push_str("  }\n\n");
        output.push_str("}\n\n");

        output
    }

    /// Generate JavaScript footer
    fn generate_javascript_footer(&self) -> String {
        use std::fmt::Write;
        let mut output = String::new();

        // Export
        output.push_str("// Export for different module systems\n");
        output.push_str("if (typeof module !== 'undefined' && module.exports) {\n");
        let _ = writeln!(output, "  module.exports = {};", self.config.class_name);
        output.push_str("} else if (typeof define === 'function' && define.amd) {\n");

        let _ = writeln!(
            output,
            "  define([], function() {{ return {}; }});",
            self.config.class_name
        );
        output.push_str("} else if (typeof window !== 'undefined') {\n");
        let _ = writeln!(
            output,
            "  window.{} = {};",
            self.config.class_name, self.config.class_name
        );
        output.push_str("}\n");

        output
    }

    /// Generate JavaScript expand method
    fn generate_javascript_expand_method() -> String {
        let mut method = String::new();

        method.push_str(
            "  /**
",
        );
        method.push_str(
            "   * Expand a CURIE to a full URI
",
        );
        method.push_str(
            "   * @param {string} curie - Compact URI (e.g., 'ex:Person')
",
        );
        method.push_str(
            "   * @returns {string} Expanded URI
",
        );
        method.push_str(
            "   * @throws {Error} If prefix is not registered
",
        );
        method.push_str(
            "   */
",
        );
        method.push_str(
            "  expand(curie) {
",
        );
        method.push_str(
            "    if (!curie.includes(':')) {
",
        );
        method.push_str(
            "      if (this._defaultPrefix && this._prefixes.has(this._defaultPrefix)) {
",
        );
        method.push_str(
            "        return this._prefixes.get(this._defaultPrefix) + curie;
",
        );
        method.push_str(
            "      }
",
        );
        method.push_str(
            "      return curie;
",
        );
        method.push_str(
            "    }
",
        );
        method.push('\n');
        method.push_str(
            "    const [prefix, localName] = curie.split(':', 2);
",
        );
        method.push('\n');
        method.push_str(
            "    if (this._prefixes.has(prefix)) {
",
        );
        method.push_str(
            "      return this._prefixes.get(prefix) + localName;
",
        );
        method.push_str(
            "    }
",
        );
        method.push('\n');
        method.push_str(
            "    throw new Error(`Unknown prefix: ${prefix}`);
",
        );
        method.push_str(
            "  }

",
        );

        method
    }

    /// Generate JavaScript methods
    fn generate_javascript_methods(&self) -> String {
        let mut methods = String::new();

        // Expand method
        methods.push_str(&Self::generate_javascript_expand_method());

        // Contract method
        methods.push_str(
            "  /**
",
        );
        methods.push_str(
            "   * Contract a URI to a CURIE if possible
",
        );
        methods.push_str(
            "   * @param {string} uri - Full URI to contract
",
        );
        methods.push_str(
            "   * @returns {string|null} CURIE if contraction possible
",
        );
        methods.push_str(
            "   */
",
        );
        methods.push_str(
            "  contract(uri) {
",
        );
        methods.push_str(
            "    let bestMatch = null;
",
        );
        methods.push_str(
            "    let bestLength = 0;
",
        );
        methods.push_str(
            "    
",
        );
        methods.push_str(
            "    for (const [namespace, prefix] of this._namespaces) {
",
        );
        methods.push_str(
            "      if (uri.startsWith(namespace) && namespace.length > bestLength) {
",
        );
        methods.push_str(
            "        bestMatch = { namespace, prefix };
",
        );
        methods.push_str(
            "        bestLength = namespace.length;
",
        );
        methods.push_str(
            "      }
",
        );
        methods.push_str(
            "    }
",
        );
        methods.push_str(
            "    
",
        );
        methods.push_str(
            "    if (bestMatch) {
",
        );
        methods.push_str(
            "      const localName = uri.substring(bestMatch.namespace.length);
",
        );
        methods.push_str(
            "      return `${bestMatch.prefix}:${localName}`;
",
        );
        methods.push_str(
            "    }
",
        );
        methods.push_str(
            "    
",
        );
        methods.push_str(
            "    return null;
",
        );
        methods.push_str(
            "  }

",
        );

        // Bind method
        methods.push_str(
            "  /**
",
        );
        methods.push_str(
            "   * Bind a new prefix to a namespace
",
        );
        methods.push_str(
            "   * @param {string} prefix - Prefix to bind
",
        );
        methods.push_str(
            "   * @param {string} namespace - Namespace URI
",
        );
        methods.push_str(
            "   */
",
        );
        methods.push_str(
            "  bind(prefix, namespace) {
",
        );
        methods.push_str(
            "    this._prefixes.set(prefix, namespace);
",
        );
        methods.push_str(
            "    this._namespaces.set(namespace, prefix);
",
        );
        methods.push_str(
            "  }

",
        );

        if self.config.include_utilities {
            // Export methods
            methods.push_str(
                "  /**
",
            );
            methods.push_str(
                "   * Export prefixes in Turtle format
",
            );
            methods.push_str(
                "   * @returns {string} Turtle prefix declarations
",
            );
            methods.push_str(
                "   */
",
            );
            methods.push_str(
                "  exportTurtle() {
",
            );
            methods.push_str(
                "    const lines = [];
",
            );
            methods.push_str(
                "    for (const [prefix, namespace] of [...this._prefixes].sort()) {
",
            );
            methods.push_str(
                "      lines.push(`@prefix ${prefix}: <${namespace}> .`);
",
            );
            methods.push_str(
                "    }
",
            );
            methods.push_str(
                "    return lines.join('\
');
",
            );
            methods.push_str(
                "  }

",
            );

            methods.push_str(
                "  /**
",
            );
            methods.push_str(
                "   * Get all prefixes
",
            );
            methods.push_str(
                "   * @returns {Object} Prefix to namespace mappings
",
            );
            methods.push_str(
                "   */
",
            );
            methods.push_str(
                "  get prefixes() {
",
            );
            methods.push_str(
                "    return Object.fromEntries(this._prefixes);
",
            );
            methods.push_str(
                "  }
",
            );
        }

        methods
    }

    /// Generate Rust namespace manager
    fn generate_rust(&self, schema: &SchemaDefinition) -> String {
        let mut output = String::new();

        output.push_str(
            "//! Namespace manager generated from LinkML schema

",
        );

        output.push_str(
            "use std::collections::HashMap;
",
        );
        if self.config.thread_safe {
            output.push_str(
                "use std::sync::{Arc, RwLock};
",
            );
        }
        output.push('\n');

        // Struct definition
        output.push_str(
            "#[derive(Debug, Clone)]
",
        );
        writeln!(output, "pub struct {} {{", self.config.class_name)
            .expect("writeln! to String should never fail");
        if self.config.thread_safe {
            output.push_str(
                "    prefixes: Arc<RwLock<HashMap<String, String>>>,
",
            );
            output.push_str(
                "    namespaces: Arc<RwLock<HashMap<String, String>>>,
",
            );
        } else {
            output.push_str(
                "    prefixes: HashMap<String, String>,
",
            );
            output.push_str(
                "    namespaces: HashMap<String, String>,
",
            );
        }
        output.push_str(
            "    default_prefix: Option<String>,
",
        );
        output.push_str(
            "}

",
        );

        // Implementation
        writeln!(output, "impl {} {{", self.config.class_name)
            .expect("writeln! to String should never fail");

        // Constructor
        output.push_str(
            "    /// Create a new namespace manager
",
        );
        output.push_str(
            "    pub fn new() -> Self {
",
        );
        output.push_str(
            "        let mut prefixes = HashMap::new();
",
        );

        // Add schema prefixes
        {
            use std::fmt::Write;
            for (prefix, expansion) in &schema.prefixes {
                let _ = writeln!(
                    output,
                    "        prefixes.insert(\"{}\".to_string(), \"{}\".to_string());",
                    prefix,
                    Self::get_prefix_reference(expansion)
                );
            }
        }

        // Add common prefixes
        output.push_str(
            "        
",
        );
        output.push_str(
            "        // Common semantic web prefixes
",
        );
        output.push_str("        prefixes.insert(\"rdf\".to_string(), \"http://www.w3.org/1999/02/22-rdf-syntax-ns#\".to_string());
");
        output.push_str("        prefixes.insert(\"rdfs\".to_string(), \"http://www.w3.org/2000/01/rdf-schema#\".to_string());
");
        output.push_str("        prefixes.insert(\"xsd\".to_string(), \"http://www.w3.org/2001/XMLSchema#\".to_string());
");
        output.push_str("        prefixes.insert(\"owl\".to_string(), \"http://www.w3.org/2002/07/owl#\".to_string());
");
        output.push_str(
            "        
",
        );

        output.push_str(
            "        let namespaces: HashMap<_, _> = prefixes
",
        );
        output.push_str(
            "            .iter()
",
        );
        output.push_str(
            "            .map(|(k, v)| (v.clone(), k.clone()))
",
        );
        output.push_str(
            "            .collect();
",
        );
        output.push_str(
            "        
",
        );

        output.push_str(
            "        Self {
",
        );
        if self.config.thread_safe {
            output.push_str(
                "            prefixes: Arc::new(RwLock::new(prefixes)),
",
            );
            output.push_str(
                "            namespaces: Arc::new(RwLock::new(namespaces)),
",
            );
        } else {
            output.push_str(
                "            prefixes,
",
            );
            output.push_str(
                "            namespaces,
",
            );
        }

        if let Some(default_prefix) = &schema.default_prefix {
            use std::fmt::Write;
            let _ = writeln!(
                output,
                "            default_prefix: Some(\"{default_prefix}\".to_string()),"
            );
        } else {
            output.push_str(
                "            default_prefix: None,
",
            );
        }
        output.push_str(
            "        }
",
        );
        output.push_str(
            "    }

",
        );

        // Core methods
        output.push_str(&self.generate_rust_methods());

        output.push_str(
            "}

",
        );

        // Default implementation
        writeln!(output, "impl Default for {} {{", self.config.class_name)
            .expect("writeln! to String should never fail");
        output.push_str(
            "    fn default() -> Self {
",
        );
        output.push_str(
            "        Self::new()
",
        );
        output.push_str(
            "    }
",
        );
        output.push_str(
            "}
",
        );

        output
    }

    /// Generate Rust methods
    fn generate_rust_methods(&self) -> String {
        let mut methods = String::new();

        // Expand method
        methods.push_str(
            "    /// Expand a CURIE to a full URI
",
        );
        methods.push_str(
            "    pub fn expand(&self, curie: &str) -> Result<String, String> {
",
        );

        if self.config.thread_safe {
            methods.push_str(
                "        let prefixes = self.prefixes.read().map_err(|_| \"Lock poisoned\")?;
",
            );
        }

        methods.push_str(
            "        if !curie.contains(':') {
",
        );
        methods.push_str(
            "            if let Some(ref default_prefix) = self.default_prefix {
",
        );
        if self.config.thread_safe {
            methods.push_str(
                "                if let Some(namespace) = prefixes.get(default_prefix) {
",
            );
        } else {
            methods.push_str(
                "                if let Some(namespace) = self.prefixes.get(default_prefix) {
",
            );
        }
        methods.push_str(
            "                    return Ok(format!(\"{}{}\", namespace, curie));
",
        );
        methods.push_str(
            "                }
",
        );
        methods.push_str(
            "            }
",
        );
        methods.push_str(
            "            return Ok(curie.to_string());
",
        );
        methods.push_str(
            "        }
",
        );
        methods.push_str(
            "        
",
        );
        methods.push_str(
            "        let parts: Vec<&str> = curie.splitn(2, ':').collect();
",
        );
        methods.push_str(
            "        if parts.len() != 2 {
",
        );
        methods.push_str(
            "            return Err(format!(\"Invalid CURIE format: {}\", curie));
",
        );
        methods.push_str(
            "        }
",
        );
        methods.push_str(
            "        
",
        );
        methods.push_str(
            "        let (prefix, local_name) = (parts[0], parts[1]);
",
        );
        methods.push_str(
            "        
",
        );
        if self.config.thread_safe {
            methods.push_str(
                "        if let Some(namespace) = prefixes.get(prefix) {
",
            );
        } else {
            methods.push_str(
                "        if let Some(namespace) = self.prefixes.get(prefix) {
",
            );
        }
        methods.push_str(
            "            Ok(format!(\"{}{}\", namespace, local_name))
",
        );
        methods.push_str(
            "        } else {
",
        );
        methods.push_str(
            "            Err(format!(\"Unknown prefix: {}\", prefix))
",
        );
        methods.push_str(
            "        }
",
        );
        methods.push_str(
            "    }

",
        );

        // Contract method
        methods.push_str(
            "    /// Contract a URI to a CURIE if possible
",
        );
        methods.push_str(
            "    pub fn contract(&self, uri: &str) -> Option<String> {
",
        );

        if self.config.thread_safe {
            methods.push_str(
                "        let namespaces = self.namespaces.read().ok()?;
",
            );
        }

        methods.push_str(
            "        let mut best_match = None;
",
        );
        methods.push_str(
            "        let mut best_length = 0;
",
        );
        methods.push_str(
            "        
",
        );

        if self.config.thread_safe {
            methods.push_str(
                "        for (namespace, prefix) in namespaces.iter() {
",
            );
        } else {
            methods.push_str(
                "        for (namespace, prefix) in &self.namespaces {
",
            );
        }
        methods.push_str(
            "            if uri.starts_with(namespace) && namespace.len() > best_length {
",
        );
        methods.push_str(
            "                best_match = Some((namespace, prefix));
",
        );
        methods.push_str(
            "                best_length = namespace.len();
",
        );
        methods.push_str(
            "            }
",
        );
        methods.push_str(
            "        }
",
        );
        methods.push_str(
            "        
",
        );
        methods.push_str(
            "        if let Some((namespace, prefix)) = best_match {
",
        );
        methods.push_str(
            "            let local_name = &uri[namespace.len()..];
",
        );
        methods.push_str(
            "            Some(format!(\"{}:{}\", prefix, local_name))
",
        );
        methods.push_str(
            "        } else {
",
        );
        methods.push_str(
            "            None
",
        );
        methods.push_str(
            "        }
",
        );
        methods.push_str(
            "    }

",
        );

        // Bind method
        methods.push_str(
            "    /// Bind a new prefix to a namespace
",
        );
        methods.push_str("    pub fn bind(&mut self, prefix: String, namespace: String) ");
        if self.config.thread_safe {
            methods.push_str(
                "-> Result<(), String> {
",
            );
            methods.push_str(
                "        let mut prefixes = self.prefixes.write().map_err(|_| \"Lock poisoned\")?;
",
            );
            methods.push_str("        let mut namespaces = self.namespaces.write().map_err(|_| \"Lock poisoned\")?;
");
            methods.push_str(
                "        
",
            );
            methods.push_str(
                "        prefixes.insert(prefix.clone(), namespace.clone());
",
            );
            methods.push_str(
                "        namespaces.insert(namespace, prefix);
",
            );
            methods.push_str(
                "        Ok(())
",
            );
        } else {
            methods.push_str(
                "{
",
            );
            methods.push_str(
                "        self.prefixes.insert(prefix.clone(), namespace.clone());
",
            );
            methods.push_str(
                "        self.namespaces.insert(namespace, prefix);
",
            );
        }
        methods.push_str(
            "    }
",
        );

        methods
    }

    /// Generate Java namespace manager
    fn generate_java(&self, schema: &SchemaDefinition) -> String {
        let mut output = String::new();

        output.push_str(
            "/**
",
        );
        output.push_str(
            " * Namespace manager generated from LinkML schema
",
        );
        output.push_str(
            " */

",
        );

        output.push_str(
            "package org.linkml.namespace;

",
        );

        output.push_str(
            "import java.util.Map;
",
        );
        output.push_str(
            "import java.util.HashMap;
",
        );
        output.push_str(
            "import java.util.Optional;
",
        );
        if self.config.thread_safe {
            output.push_str(
                "import java.util.concurrent.ConcurrentHashMap;
",
            );
        }
        output.push('\n');

        // Class definition
        writeln!(output, "public class {} {{", self.config.class_name)
            .expect("writeln! to String should never fail");

        if self.config.thread_safe {
            output.push_str(
                "    private final Map<String, String> prefixes = new ConcurrentHashMap<>();
",
            );
            output.push_str(
                "    private final Map<String, String> namespaces = new ConcurrentHashMap<>();
",
            );
        } else {
            output.push_str(
                "    private final Map<String, String> prefixes = new HashMap<>();
",
            );
            output.push_str(
                "    private final Map<String, String> namespaces = new HashMap<>();
",
            );
        }
        output.push_str(
            "    private final String defaultPrefix;

",
        );

        // Constructor

        let _ = writeln!(output, "    public {}() {{", self.config.class_name);

        // Add schema prefixes
        for (prefix, expansion) in &schema.prefixes {
            use std::fmt::Write;
            let _ = writeln!(
                output,
                "        prefixes.put(\"{}\", \"{}\");",
                prefix,
                Self::get_prefix_reference(expansion)
            );
        }

        // Add common prefixes
        output.push_str(
            "        
",
        );
        output.push_str(
            "        // Common semantic web prefixes
",
        );
        output.push_str(
            "        prefixes.put(\"rdf\", \"http://www.w3.org/1999/02/22-rdf-syntax-ns#\");
",
        );
        output.push_str(
            "        prefixes.put(\"rdfs\", \"http://www.w3.org/2000/01/rdf-schema#\");
",
        );
        output.push_str(
            "        prefixes.put(\"xsd\", \"http://www.w3.org/2001/XMLSchema#\");
",
        );
        output.push_str(
            "        prefixes.put(\"owl\", \"http://www.w3.org/2002/07/owl#\");
",
        );
        output.push_str(
            "        
",
        );

        output.push_str(
            "        // Build reverse mapping
",
        );
        output.push_str(
            "        prefixes.forEach((k, v) -> namespaces.put(v, k));
",
        );
        output.push_str(
            "        
",
        );

        if let Some(default_prefix) = &schema.default_prefix {
            writeln!(output, "        this.defaultPrefix = \"{default_prefix}\";")
                .expect("LinkML operation should succeed");
        } else {
            output.push_str(
                "        this.defaultPrefix = null;
",
            );
        }
        output.push_str(
            "    }

",
        );

        // Core methods
        output.push_str(&Self::generate_java_methods());

        output.push_str(
            "}
",
        );

        output
    }

    /// Generate Java methods
    fn generate_java_methods() -> String {
        let mut methods = String::new();

        // Expand method
        methods.push_str(
            "    /**
",
        );
        methods.push_str(
            "     * Expand a CURIE to a full URI
",
        );
        methods.push_str(
            "     * @param curie Compact URI (e.g., 'ex:Person')
",
        );
        methods.push_str(
            "     * @return Expanded URI
",
        );
        methods.push_str(
            "     * @throws IllegalArgumentException If prefix is not registered
",
        );
        methods.push_str(
            "     */
",
        );
        methods.push_str(
            "    public String expand(String curie) {
",
        );
        methods.push_str(
            "        if (!curie.contains(\":\")) {
",
        );
        methods.push_str(
            "            if (defaultPrefix != null && prefixes.containsKey(defaultPrefix)) {
",
        );
        methods.push_str(
            "                return prefixes.get(defaultPrefix) + curie;
",
        );
        methods.push_str(
            "            }
",
        );
        methods.push_str(
            "            return curie;
",
        );
        methods.push_str(
            "        }
",
        );
        methods.push_str(
            "        
",
        );
        methods.push_str(
            "        String[] parts = curie.split(\":\", 2);
",
        );
        methods.push_str(
            "        String prefix = parts[0];
",
        );
        methods.push_str(
            "        String localName = parts[1];
",
        );
        methods.push_str(
            "        
",
        );
        methods.push_str(
            "        if (prefixes.containsKey(prefix)) {
",
        );
        methods.push_str(
            "            return prefixes.get(prefix) + localName;
",
        );
        methods.push_str(
            "        }
",
        );
        methods.push_str(
            "        
",
        );
        methods.push_str(
            "        throw new IllegalArgumentException(\"Unknown prefix: \" + prefix);
",
        );
        methods.push_str(
            "    }

",
        );

        // Contract method
        methods.push_str(
            "    /**
",
        );
        methods.push_str(
            "     * Contract a URI to a CURIE if possible
",
        );
        methods.push_str(
            "     * @param uri Full URI to contract
",
        );
        methods.push_str(
            "     * @return CURIE if contraction possible
",
        );
        methods.push_str(
            "     */
",
        );
        methods.push_str(
            "    public Optional<String> contract(String uri) {
",
        );
        methods.push_str(
            "        String bestNamespace = null;
",
        );
        methods.push_str(
            "        int bestLength = 0;
",
        );
        methods.push_str(
            "        
",
        );
        methods.push_str(
            "        for (Map.Entry<String, String> entry : namespaces.entrySet()) {
",
        );
        methods.push_str(
            "            String namespace = entry.getKey();
",
        );
        methods.push_str(
            "            if (uri.startsWith(namespace) && namespace.length() > bestLength) {
",
        );
        methods.push_str(
            "                bestNamespace = namespace;
",
        );
        methods.push_str(
            "                bestLength = namespace.length();
",
        );
        methods.push_str(
            "            }
",
        );
        methods.push_str(
            "        }
",
        );
        methods.push_str(
            "        
",
        );
        methods.push_str(
            "        if (bestNamespace != null) {
",
        );
        methods.push_str(
            "            String prefix = namespaces.get(bestNamespace);
",
        );
        methods.push_str(
            "            String localName = uri.substring(bestNamespace.length());
",
        );
        methods.push_str(
            "            return Optional.of(prefix + \":\" + localName);
",
        );
        methods.push_str(
            "        }
",
        );
        methods.push_str(
            "        
",
        );
        methods.push_str(
            "        return Optional.empty();
",
        );
        methods.push_str(
            "    }
",
        );

        methods
    }

    /// Generate Go namespace manager
    fn generate_go(&self, schema: &SchemaDefinition) -> String {
        let mut output = String::new();

        output.push_str(
            "// Package namespace provides namespace management for LinkML schemas
",
        );
        output.push_str(
            "package namespace

",
        );

        output.push_str(
            "import (
",
        );
        output.push_str(
            "\t\"fmt\"
",
        );
        output.push_str(
            "\t\"strings\"
",
        );
        if self.config.thread_safe {
            output.push_str(
                "\t\"sync\"
",
            );
        }
        output.push_str(
            ")

",
        );

        // Struct definition
        output.push_str(
            "// Manager manages namespace prefixes and URI expansion/contraction
",
        );
        output.push_str(
            "type Manager struct {
",
        );
        if self.config.thread_safe {
            output.push_str(
                "\tmu sync.RWMutex
",
            );
        }
        output.push_str(
            "\tprefixes map[string]string
",
        );
        output.push_str(
            "\tnamespaces map[string]string
",
        );
        output.push_str(
            "\tdefaultPrefix string
",
        );
        output.push_str(
            "}

",
        );

        // Constructor
        output.push_str(
            "// NewManager creates a new namespace manager
",
        );
        output.push_str(
            "func NewManager() *Manager {
",
        );
        output.push_str(
            "\tm := &Manager{
",
        );
        output.push_str(
            "\t\tprefixes: make(map[string]string),
",
        );
        output.push_str(
            "\t\tnamespaces: make(map[string]string),
",
        );
        output.push_str(
            "\t}

",
        );

        // Add schema prefixes
        for (prefix, expansion) in &schema.prefixes {
            use std::fmt::Write;
            let _ = writeln!(
                output,
                "\tm.prefixes[\"{}\"] = \"{}\"",
                prefix,
                Self::get_prefix_reference(expansion)
            );
        }
        if !schema.prefixes.is_empty() {
            output.push('\n');
        }

        // Add common prefixes
        output.push_str(
            "\t// Common semantic web prefixes
",
        );
        output.push_str(
            "\tm.prefixes[\"rdf\"] = \"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"
",
        );
        output.push_str(
            "\tm.prefixes[\"rdfs\"] = \"http://www.w3.org/2000/01/rdf-schema#\"
",
        );
        output.push_str(
            "\tm.prefixes[\"xsd\"] = \"http://www.w3.org/2001/XMLSchema#\"
",
        );
        output.push_str(
            "\tm.prefixes[\"owl\"] = \"http://www.w3.org/2002/07/owl#\"

",
        );

        output.push_str(
            "\t// Build reverse mapping
",
        );
        output.push_str(
            "\tfor prefix, namespace := range m.prefixes {
",
        );
        output.push_str(
            "\t\tm.namespaces[namespace] = prefix
",
        );
        output.push_str(
            "\t}

",
        );

        if let Some(default_prefix) = &schema.default_prefix {
            writeln!(output, "\tm.defaultPrefix = \"{default_prefix}\"")
                .expect("writeln! to String should never fail");
        }

        output.push_str(
            "\treturn m
",
        );
        output.push_str(
            "}

",
        );

        // Core methods
        output.push_str(&self.generate_go_methods());

        output
    }

    /// Generate Go methods
    fn generate_go_methods(&self) -> String {
        let mut methods = String::new();

        // Expand method
        methods.push_str(
            "// Expand expands a CURIE to a full URI
",
        );
        methods.push_str(
            "func (m *Manager) Expand(curie string) (string, error) {
",
        );
        if self.config.thread_safe {
            methods.push_str(
                "\tm.mu.RLock()
",
            );
            methods.push_str(
                "\tdefer m.mu.RUnlock()

",
            );
        }

        methods.push_str(
            "\tif !strings.Contains(curie, \":\") {
",
        );
        methods.push_str(
            "\t\tif m.defaultPrefix != \"\" {
",
        );
        methods.push_str(
            "\t\t\tif namespace, ok := m.prefixes[m.defaultPrefix]; ok {
",
        );
        methods.push_str(
            "\t\t\t\treturn namespace + curie, nil
",
        );
        methods.push_str(
            "\t\t\t}
",
        );
        methods.push_str(
            "\t\t}
",
        );
        methods.push_str(
            "\t\treturn curie, nil
",
        );
        methods.push_str(
            "\t}

",
        );

        methods.push_str(
            "\tparts := strings.SplitN(curie, \":\", 2)
",
        );
        methods.push_str(
            "\tif len(parts) != 2 {
",
        );
        methods.push_str(
            "\t\treturn \"\", fmt.Errorf(\"invalid CURIE format: %s\", curie)
",
        );
        methods.push_str(
            "\t}

",
        );

        methods.push_str(
            "\tprefix, localName := parts[0], parts[1]

",
        );

        methods.push_str(
            "\tif namespace, ok := m.prefixes[prefix]; ok {
",
        );
        methods.push_str(
            "\t\treturn namespace + localName, nil
",
        );
        methods.push_str(
            "\t}

",
        );

        methods.push_str(
            "\treturn \"\", fmt.Errorf(\"unknown prefix: %s\", prefix)
",
        );
        methods.push_str(
            "}

",
        );

        // Contract method
        methods.push_str(
            "// Contract contracts a URI to a CURIE if possible
",
        );
        methods.push_str(
            "func (m *Manager) Contract(uri string) string {
",
        );
        if self.config.thread_safe {
            methods.push_str(
                "\tm.mu.RLock()
",
            );
            methods.push_str(
                "\tdefer m.mu.RUnlock()

",
            );
        }

        methods.push_str(
            "\tvar bestNamespace string
",
        );
        methods.push_str(
            "\tvar bestPrefix string
",
        );
        methods.push_str(
            "\tbestLength := 0

",
        );

        methods.push_str(
            "\tfor namespace, prefix := range m.namespaces {
",
        );
        methods.push_str(
            "\t\tif strings.HasPrefix(uri, namespace) && len(namespace) > bestLength {
",
        );
        methods.push_str(
            "\t\t\tbestNamespace = namespace
",
        );
        methods.push_str(
            "\t\t\tbestPrefix = prefix
",
        );
        methods.push_str(
            "\t\t\tbestLength = len(namespace)
",
        );
        methods.push_str(
            "\t\t}
",
        );
        methods.push_str(
            "\t}

",
        );

        methods.push_str(
            "\tif bestNamespace != \"\" {
",
        );
        methods.push_str(
            "\t\tlocalName := uri[len(bestNamespace):]
",
        );
        methods.push_str(
            "\t\treturn fmt.Sprintf(\"%s:%s\", bestPrefix, localName)
",
        );
        methods.push_str(
            "\t}

",
        );

        methods.push_str(
            "\treturn uri
",
        );
        methods.push_str(
            "}
",
        );

        methods
    }
}

impl Generator for NamespaceManagerGenerator {
    fn name(&self) -> &'static str {
        "namespace_manager"
    }

    fn description(&self) -> &'static str {
        "Generate namespace manager utilities for handling URI prefixes and namespace resolution"
    }

    fn validate_schema(&self, schema: &SchemaDefinition) -> linkml_core::error::Result<()> {
        // Validate that the schema has required fields for namespace management
        if schema.name.is_empty() {
            return Err(LinkMLError::data_validation(
                "Schema must have a name for namespace manager generation",
            ));
        }

        // Validate prefixes if present
        for (prefix_name, prefix_def) in &schema.prefixes {
            if prefix_name.is_empty() {
                return Err(LinkMLError::data_validation("Prefix name cannot be empty"));
            }
            match prefix_def {
                PrefixDefinition::Simple(uri) => {
                    if uri.is_empty() {
                        return Err(LinkMLError::data_validation(format!(
                            "Prefix '{prefix_name}' has empty URI"
                        )));
                    }
                }
                PrefixDefinition::Complex {
                    prefix_prefix,
                    prefix_reference,
                } => {
                    if prefix_prefix.is_empty() {
                        return Err(LinkMLError::data_validation(format!(
                            "Prefix '{prefix_name}' has empty expansion"
                        )));
                    }
                    // Validate prefix_reference if provided
                    if let Some(ref_value) = prefix_reference
                        && ref_value.is_empty()
                    {
                        return Err(LinkMLError::data_validation(format!(
                            "Prefix '{prefix_name}' has empty reference value"
                        )));
                    }
                }
            }
        }

        Ok(())
    }

    fn generate(&self, schema: &SchemaDefinition) -> linkml_core::error::Result<String> {
        Ok(self.generate_manager(schema))
    }

    fn get_file_extension(&self) -> &str {
        match self.config.target_language {
            TargetLanguage::Python => "py",
            TargetLanguage::JavaScript => "js",
            TargetLanguage::Rust => "rs",
            TargetLanguage::Java => "java",
            TargetLanguage::Go => "go",
        }
    }

    fn get_default_filename(&self) -> &'static str {
        "namespace_manager"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::types::SchemaDefinition;

    #[test]
    fn test_namespace_manager_generation() -> anyhow::Result<(), LinkMLError> {
        let mut schema = SchemaDefinition::default();
        schema.name = "TestSchema".to_string();

        // Add prefixes
        use indexmap::IndexMap;
        use linkml_core::prelude::PrefixDefinition;
        let mut prefixes = IndexMap::new();
        prefixes.insert(
            "ex".to_string(),
            PrefixDefinition::Complex {
                prefix_prefix: "ex".to_string(),
                prefix_reference: Some("https://example.com/".to_string()),
            },
        );
        schema.prefixes = prefixes;
        schema.default_prefix = Some("ex".to_string());

        // Test Python generation
        let config = NamespaceManagerGeneratorConfig::default();
        let generator = NamespaceManagerGenerator::new(config);
        let result = generator
            .generate(&schema)
            .expect("should generate namespace manager: {}");

        assert!(result.contains("class NamespaceManager:"));
        assert!(result.contains("def expand("));
        assert!(result.contains("def contract("));
        Ok(())
    }
}
