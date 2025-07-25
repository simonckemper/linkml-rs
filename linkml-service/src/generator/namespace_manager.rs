//! Namespace manager generator for LinkML schemas
//!
//! This module generates namespace management utilities from LinkML schemas,
//! including prefix expansion/contraction, URI validation, and namespace resolution.

use crate::error::LinkMLError;
use crate::generator::traits::{Generator, GeneratorConfig};
use linkml_core::schema::{Schema, Prefix};
use std::collections::{HashMap, HashSet};

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
}

impl NamespaceManagerGenerator {
    /// Create a new namespace manager generator
    pub fn new(config: NamespaceManagerGeneratorConfig) -> Self {
        Self { config }
    }
    
    /// Generate namespace manager for the configured language
    fn generate_manager(&self, schema: &Schema) -> Result<String, LinkMLError> {
        match self.config.target_language {
            TargetLanguage::Python => self.generate_python(schema),
            TargetLanguage::JavaScript => self.generate_javascript(schema),
            TargetLanguage::Rust => self.generate_rust(schema),
            TargetLanguage::Java => self.generate_java(schema),
            TargetLanguage::Go => self.generate_go(schema),
        }
    }
    
    /// Generate Python namespace manager
    fn generate_python(&self, schema: &Schema) -> Result<String, LinkMLError> {
        let mut output = String::new();
        
        // Header
        output.push_str("#!/usr/bin/env python3\n");
        output.push_str("\"\"\"Namespace manager generated from LinkML schema\"\"\"\n\n");
        
        // Imports
        output.push_str("from typing import Dict, Optional, List, Tuple, Set\n");
        output.push_str("import re\n");
        if self.config.thread_safe {
            output.push_str("import threading\n");
        }
        output.push_str("\n\n");
        
        // Class definition
        output.push_str(&format!("class {}:\n", self.config.class_name));
        output.push_str("    \"\"\"Manages namespace prefixes and URI expansion/contraction\"\"\"\n\n");
        
        // Constructor
        output.push_str("    def __init__(self):\n");
        output.push_str("        \"\"\"Initialize namespace manager with predefined prefixes\"\"\"\n");
        if self.config.thread_safe {
            output.push_str("        self._lock = threading.RLock()\n");
        }
        
        // Initialize prefix mappings
        output.push_str("        self._prefixes: Dict[str, str] = {\n");
        if let Some(prefixes) = &schema.prefixes {
            for (prefix, expansion) in prefixes {
                output.push_str(&format!(
                    "            '{}': '{}',\n",
                    prefix, expansion.prefix_reference
                ));
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
        output.push_str("        self._namespaces: Dict[str, str] = {\n");
        output.push_str("            v: k for k, v in self._prefixes.items()\n");
        output.push_str("        }\n");
        
        // Default prefix
        if let Some(default_prefix) = &schema.default_prefix {
            output.push_str(&format!("        self._default_prefix = '{}'\n", default_prefix));
        } else {
            output.push_str("        self._default_prefix = None\n");
        }
        output.push_str("\n");
        
        // Core methods
        output.push_str(&self.generate_python_expand_method());
        output.push_str(&self.generate_python_contract_method());
        output.push_str(&self.generate_python_bind_method());
        
        if self.config.include_validation {
            output.push_str(&self.generate_python_validation_methods());
        }
        
        if self.config.include_utilities {
            output.push_str(&self.generate_python_utility_methods());
        }
        
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
        output.push_str("\n");
        
        output.push_str("    @property\n");
        output.push_str("    def namespaces(self) -> Dict[str, str]:\n");
        output.push_str("        \"\"\"Get copy of namespace mappings\"\"\"\n");
        if self.config.thread_safe {
            output.push_str("        with self._lock:\n");
            output.push_str("            return self._namespaces.copy()\n");
        } else {
            output.push_str("        return self._namespaces.copy()\n");
        }
        output.push_str("\n");
        
        Ok(output)
    }
    
    /// Generate Python expand method
    fn generate_python_expand_method(&self) -> String {
        let mut method = String::new();
        
        method.push_str("    def expand(self, curie: str) -> str:\n");
        method.push_str("        \"\"\"Expand a CURIE to a full URI\n");
        method.push_str("        \n");
        method.push_str("        Args:\n");
        method.push_str("            curie: Compact URI (e.g., 'ex:Person')\n");
        method.push_str("            \n");
        method.push_str("        Returns:\n");
        method.push_str("            Expanded URI\n");
        method.push_str("            \n");
        method.push_str("        Raises:\n");
        method.push_str("            ValueError: If prefix is not registered\n");
        method.push_str("        \"\"\"\n");
        
        if self.config.thread_safe {
            method.push_str("        with self._lock:\n");
            method.push_str("            ");
        }
        
        method.push_str("        if ':' not in curie:\n");
        method.push_str("            # Use default prefix if available\n");
        method.push_str("            if self._default_prefix and self._default_prefix in self._prefixes:\n");
        method.push_str("                return self._prefixes[self._default_prefix] + curie\n");
        method.push_str("            return curie\n");
        method.push_str("        \n");
        method.push_str("        prefix, local_name = curie.split(':', 1)\n");
        method.push_str("        \n");
        method.push_str("        if prefix in self._prefixes:\n");
        method.push_str("            return self._prefixes[prefix] + local_name\n");
        method.push_str("        \n");
        method.push_str("        raise ValueError(f\"Unknown prefix: {prefix}\")\n");
        method.push_str("\n");
        
        method
    }
    
    /// Generate Python contract method
    fn generate_python_contract_method(&self) -> String {
        let mut method = String::new();
        
        method.push_str("    def contract(self, uri: str) -> Optional[str]:\n");
        method.push_str("        \"\"\"Contract a URI to a CURIE if possible\n");
        method.push_str("        \n");
        method.push_str("        Args:\n");
        method.push_str("            uri: Full URI to contract\n");
        method.push_str("            \n");
        method.push_str("        Returns:\n");
        method.push_str("            CURIE if contraction possible, otherwise None\n");
        method.push_str("        \"\"\"\n");
        
        if self.config.thread_safe {
            method.push_str("        with self._lock:\n");
            method.push_str("            ");
        }
        
        method.push_str("        # Find the longest matching namespace\n");
        method.push_str("        best_match = None\n");
        method.push_str("        best_length = 0\n");
        method.push_str("        \n");
        method.push_str("        for namespace, prefix in self._namespaces.items():\n");
        method.push_str("            if uri.startswith(namespace) and len(namespace) > best_length:\n");
        method.push_str("                best_match = (namespace, prefix)\n");
        method.push_str("                best_length = len(namespace)\n");
        method.push_str("        \n");
        method.push_str("        if best_match:\n");
        method.push_str("            namespace, prefix = best_match\n");
        method.push_str("            local_name = uri[len(namespace):]\n");
        method.push_str("            return f\"{prefix}:{local_name}\"\n");
        method.push_str("        \n");
        method.push_str("        return None\n");
        method.push_str("\n");
        
        method
    }
    
    /// Generate Python bind method
    fn generate_python_bind_method(&self) -> String {
        let mut method = String::new();
        
        method.push_str("    def bind(self, prefix: str, namespace: str) -> None:\n");
        method.push_str("        \"\"\"Bind a new prefix to a namespace\n");
        method.push_str("        \n");
        method.push_str("        Args:\n");
        method.push_str("            prefix: Prefix to bind\n");
        method.push_str("            namespace: Namespace URI\n");
        method.push_str("        \"\"\"\n");
        
        if self.config.thread_safe {
            method.push_str("        with self._lock:\n");
            method.push_str("            ");
        }
        
        method.push_str("        self._prefixes[prefix] = namespace\n");
        method.push_str("        self._namespaces[namespace] = prefix\n");
        method.push_str("\n");
        
        method
    }
    
    /// Generate Python validation methods
    fn generate_python_validation_methods(&self) -> String {
        let mut methods = String::new();
        
        // Validate URI
        methods.push_str("    def is_valid_uri(self, uri: str) -> bool:\n");
        methods.push_str("        \"\"\"Check if a string is a valid URI\"\"\"\n");
        methods.push_str("        uri_pattern = re.compile(\n");
        methods.push_str("            r'^[a-zA-Z][a-zA-Z0-9+.-]*:'\n");
        methods.push_str("            r'(?://(?:[a-zA-Z0-9._~-]|%[0-9A-Fa-f]{2})*'\n");
        methods.push_str("            r'(?::[0-9]*)?'\n");
        methods.push_str("            r'(?:/(?:[a-zA-Z0-9._~-]|%[0-9A-Fa-f]{2})*)*)?'\n");
        methods.push_str("        )\n");
        methods.push_str("        return bool(uri_pattern.match(uri))\n");
        methods.push_str("\n");
        
        // Validate CURIE
        methods.push_str("    def is_valid_curie(self, curie: str) -> bool:\n");
        methods.push_str("        \"\"\"Check if a string is a valid CURIE\"\"\"\n");
        methods.push_str("        if ':' not in curie:\n");
        methods.push_str("            return False\n");
        methods.push_str("        prefix, local = curie.split(':', 1)\n");
        methods.push_str("        return prefix in self._prefixes\n");
        methods.push_str("\n");
        
        // Validate prefix
        methods.push_str("    def is_valid_prefix(self, prefix: str) -> bool:\n");
        methods.push_str("        \"\"\"Check if a prefix is valid\"\"\"\n");
        methods.push_str("        return bool(re.match(r'^[a-zA-Z_][a-zA-Z0-9_-]*$', prefix))\n");
        methods.push_str("\n");
        
        methods
    }
    
    /// Generate Python utility methods
    fn generate_python_utility_methods(&self) -> String {
        let mut methods = String::new();
        
        // Get all CURIEs for a namespace
        methods.push_str("    def get_curies_for_namespace(self, namespace: str) -> List[str]:\n");
        methods.push_str("        \"\"\"Get all registered CURIEs for a namespace\"\"\"\n");
        if self.config.thread_safe {
            methods.push_str("        with self._lock:\n");
            methods.push_str("            ");
        }
        methods.push_str("        return [f\"{p}:\" for p, ns in self._prefixes.items() if ns == namespace]\n");
        methods.push_str("\n");
        
        // Normalize URI
        methods.push_str("    def normalize(self, uri_or_curie: str) -> str:\n");
        methods.push_str("        \"\"\"Normalize a URI or CURIE to full URI form\"\"\"\n");
        methods.push_str("        if self.is_valid_curie(uri_or_curie):\n");
        methods.push_str("            return self.expand(uri_or_curie)\n");
        methods.push_str("        return uri_or_curie\n");
        methods.push_str("\n");
        
        // Export to different formats
        methods.push_str("    def export_turtle(self) -> str:\n");
        methods.push_str("        \"\"\"Export prefixes in Turtle format\"\"\"\n");
        methods.push_str("        lines = []\n");
        if self.config.thread_safe {
            methods.push_str("        with self._lock:\n");
            methods.push_str("            ");
        }
        methods.push_str("        for prefix, namespace in sorted(self._prefixes.items()):\n");
        methods.push_str("            lines.append(f\"@prefix {prefix}: <{namespace}> .\")\n");
        methods.push_str("        return '\\n'.join(lines)\n");
        methods.push_str("\n");
        
        methods.push_str("    def export_sparql(self) -> str:\n");
        methods.push_str("        \"\"\"Export prefixes in SPARQL format\"\"\"\n");
        methods.push_str("        lines = []\n");
        if self.config.thread_safe {
            methods.push_str("        with self._lock:\n");
            methods.push_str("            ");
        }
        methods.push_str("        for prefix, namespace in sorted(self._prefixes.items()):\n");
        methods.push_str("            lines.append(f\"PREFIX {prefix}: <{namespace}>\")\n");
        methods.push_str("        return '\\n'.join(lines)\n");
        methods.push_str("\n");
        
        methods
    }
    
    /// Generate JavaScript namespace manager
    fn generate_javascript(&self, schema: &Schema) -> Result<String, LinkMLError> {
        let mut output = String::new();
        
        output.push_str("/**\n");
        output.push_str(" * Namespace manager generated from LinkML schema\n");
        output.push_str(" */\n\n");
        
        // Class definition
        output.push_str(&format!("class {} {{\n", self.config.class_name));
        
        // Constructor
        output.push_str("  constructor() {\n");
        output.push_str("    /**\n");
        output.push_str("     * @type {Map<string, string>}\n");
        output.push_str("     * @private\n");
        output.push_str("     */\n");
        output.push_str("    this._prefixes = new Map([\n");
        
        // Add schema prefixes
        if let Some(prefixes) = &schema.prefixes {
            for (prefix, expansion) in prefixes {
                output.push_str(&format!(
                    "      ['{}', '{}'],\n",
                    prefix, expansion.prefix_reference
                ));
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
            output.push_str(&format!("    this._defaultPrefix = '{}';\n", default_prefix));
        } else {
            output.push_str("    this._defaultPrefix = null;\n");
        }
        output.push_str("  }\n\n");
        
        // Core methods
        output.push_str(&self.generate_javascript_methods());
        
        output.push_str("}\n\n");
        
        // Export
        output.push_str("// Export for different module systems\n");
        output.push_str("if (typeof module !== 'undefined' && module.exports) {\n");
        output.push_str(&format!("  module.exports = {};\n", self.config.class_name));
        output.push_str("} else if (typeof define === 'function' && define.amd) {\n");
        output.push_str(&format!("  define([], function() {{ return {}; }});\n", self.config.class_name));
        output.push_str("} else if (typeof window !== 'undefined') {\n");
        output.push_str(&format!("  window.{} = {};\n", self.config.class_name, self.config.class_name));
        output.push_str("}\n");
        
        Ok(output)
    }
    
    /// Generate JavaScript methods
    fn generate_javascript_methods(&self) -> String {
        let mut methods = String::new();
        
        // Expand method
        methods.push_str("  /**\n");
        methods.push_str("   * Expand a CURIE to a full URI\n");
        methods.push_str("   * @param {string} curie - Compact URI (e.g., 'ex:Person')\n");
        methods.push_str("   * @returns {string} Expanded URI\n");
        methods.push_str("   * @throws {Error} If prefix is not registered\n");
        methods.push_str("   */\n");
        methods.push_str("  expand(curie) {\n");
        methods.push_str("    if (!curie.includes(':')) {\n");
        methods.push_str("      if (this._defaultPrefix && this._prefixes.has(this._defaultPrefix)) {\n");
        methods.push_str("        return this._prefixes.get(this._defaultPrefix) + curie;\n");
        methods.push_str("      }\n");
        methods.push_str("      return curie;\n");
        methods.push_str("    }\n");
        methods.push_str("    \n");
        methods.push_str("    const [prefix, localName] = curie.split(':', 2);\n");
        methods.push_str("    \n");
        methods.push_str("    if (this._prefixes.has(prefix)) {\n");
        methods.push_str("      return this._prefixes.get(prefix) + localName;\n");
        methods.push_str("    }\n");
        methods.push_str("    \n");
        methods.push_str("    throw new Error(`Unknown prefix: ${prefix}`);\n");
        methods.push_str("  }\n\n");
        
        // Contract method
        methods.push_str("  /**\n");
        methods.push_str("   * Contract a URI to a CURIE if possible\n");
        methods.push_str("   * @param {string} uri - Full URI to contract\n");
        methods.push_str("   * @returns {string|null} CURIE if contraction possible\n");
        methods.push_str("   */\n");
        methods.push_str("  contract(uri) {\n");
        methods.push_str("    let bestMatch = null;\n");
        methods.push_str("    let bestLength = 0;\n");
        methods.push_str("    \n");
        methods.push_str("    for (const [namespace, prefix] of this._namespaces) {\n");
        methods.push_str("      if (uri.startsWith(namespace) && namespace.length > bestLength) {\n");
        methods.push_str("        bestMatch = { namespace, prefix };\n");
        methods.push_str("        bestLength = namespace.length;\n");
        methods.push_str("      }\n");
        methods.push_str("    }\n");
        methods.push_str("    \n");
        methods.push_str("    if (bestMatch) {\n");
        methods.push_str("      const localName = uri.substring(bestMatch.namespace.length);\n");
        methods.push_str("      return `${bestMatch.prefix}:${localName}`;\n");
        methods.push_str("    }\n");
        methods.push_str("    \n");
        methods.push_str("    return null;\n");
        methods.push_str("  }\n\n");
        
        // Bind method
        methods.push_str("  /**\n");
        methods.push_str("   * Bind a new prefix to a namespace\n");
        methods.push_str("   * @param {string} prefix - Prefix to bind\n");
        methods.push_str("   * @param {string} namespace - Namespace URI\n");
        methods.push_str("   */\n");
        methods.push_str("  bind(prefix, namespace) {\n");
        methods.push_str("    this._prefixes.set(prefix, namespace);\n");
        methods.push_str("    this._namespaces.set(namespace, prefix);\n");
        methods.push_str("  }\n\n");
        
        if self.config.include_utilities {
            // Export methods
            methods.push_str("  /**\n");
            methods.push_str("   * Export prefixes in Turtle format\n");
            methods.push_str("   * @returns {string} Turtle prefix declarations\n");
            methods.push_str("   */\n");
            methods.push_str("  exportTurtle() {\n");
            methods.push_str("    const lines = [];\n");
            methods.push_str("    for (const [prefix, namespace] of [...this._prefixes].sort()) {\n");
            methods.push_str("      lines.push(`@prefix ${prefix}: <${namespace}> .`);\n");
            methods.push_str("    }\n");
            methods.push_str("    return lines.join('\\n');\n");
            methods.push_str("  }\n\n");
            
            methods.push_str("  /**\n");
            methods.push_str("   * Get all prefixes\n");
            methods.push_str("   * @returns {Object} Prefix to namespace mappings\n");
            methods.push_str("   */\n");
            methods.push_str("  get prefixes() {\n");
            methods.push_str("    return Object.fromEntries(this._prefixes);\n");
            methods.push_str("  }\n");
        }
        
        methods
    }
    
    /// Generate Rust namespace manager
    fn generate_rust(&self, schema: &Schema) -> Result<String, LinkMLError> {
        let mut output = String::new();
        
        output.push_str("//! Namespace manager generated from LinkML schema\n\n");
        
        output.push_str("use std::collections::HashMap;\n");
        if self.config.thread_safe {
            output.push_str("use std::sync::{Arc, RwLock};\n");
        }
        output.push_str("\n");
        
        // Struct definition
        output.push_str("#[derive(Debug, Clone)]\n");
        output.push_str(&format!("pub struct {} {{\n", self.config.class_name));
        if self.config.thread_safe {
            output.push_str("    prefixes: Arc<RwLock<HashMap<String, String>>>,\n");
            output.push_str("    namespaces: Arc<RwLock<HashMap<String, String>>>,\n");
        } else {
            output.push_str("    prefixes: HashMap<String, String>,\n");
            output.push_str("    namespaces: HashMap<String, String>,\n");
        }
        output.push_str("    default_prefix: Option<String>,\n");
        output.push_str("}\n\n");
        
        // Implementation
        output.push_str(&format!("impl {} {{\n", self.config.class_name));
        
        // Constructor
        output.push_str("    /// Create a new namespace manager\n");
        output.push_str("    pub fn new() -> Self {\n");
        output.push_str("        let mut prefixes = HashMap::new();\n");
        
        // Add schema prefixes
        if let Some(schema_prefixes) = &schema.prefixes {
            for (prefix, expansion) in schema_prefixes {
                output.push_str(&format!(
                    "        prefixes.insert(\"{}\".to_string(), \"{}\".to_string());\n",
                    prefix, expansion.prefix_reference
                ));
            }
        }
        
        // Add common prefixes
        output.push_str("        \n");
        output.push_str("        // Common semantic web prefixes\n");
        output.push_str("        prefixes.insert(\"rdf\".to_string(), \"http://www.w3.org/1999/02/22-rdf-syntax-ns#\".to_string());\n");
        output.push_str("        prefixes.insert(\"rdfs\".to_string(), \"http://www.w3.org/2000/01/rdf-schema#\".to_string());\n");
        output.push_str("        prefixes.insert(\"xsd\".to_string(), \"http://www.w3.org/2001/XMLSchema#\".to_string());\n");
        output.push_str("        prefixes.insert(\"owl\".to_string(), \"http://www.w3.org/2002/07/owl#\".to_string());\n");
        output.push_str("        \n");
        
        output.push_str("        let namespaces: HashMap<_, _> = prefixes\n");
        output.push_str("            .iter()\n");
        output.push_str("            .map(|(k, v)| (v.clone(), k.clone()))\n");
        output.push_str("            .collect();\n");
        output.push_str("        \n");
        
        output.push_str("        Self {\n");
        if self.config.thread_safe {
            output.push_str("            prefixes: Arc::new(RwLock::new(prefixes)),\n");
            output.push_str("            namespaces: Arc::new(RwLock::new(namespaces)),\n");
        } else {
            output.push_str("            prefixes,\n");
            output.push_str("            namespaces,\n");
        }
        
        if let Some(default_prefix) = &schema.default_prefix {
            output.push_str(&format!("            default_prefix: Some(\"{}\".to_string()),\n", default_prefix));
        } else {
            output.push_str("            default_prefix: None,\n");
        }
        output.push_str("        }\n");
        output.push_str("    }\n\n");
        
        // Core methods
        output.push_str(&self.generate_rust_methods());
        
        output.push_str("}\n\n");
        
        // Default implementation
        output.push_str(&format!("impl Default for {} {{\n", self.config.class_name));
        output.push_str("    fn default() -> Self {\n");
        output.push_str("        Self::new()\n");
        output.push_str("    }\n");
        output.push_str("}\n");
        
        Ok(output)
    }
    
    /// Generate Rust methods
    fn generate_rust_methods(&self) -> String {
        let mut methods = String::new();
        
        // Expand method
        methods.push_str("    /// Expand a CURIE to a full URI\n");
        methods.push_str("    pub fn expand(&self, curie: &str) -> Result<String, String> {\n");
        
        if self.config.thread_safe {
            methods.push_str("        let prefixes = self.prefixes.read().map_err(|_| \"Lock poisoned\")?;\n");
        }
        
        methods.push_str("        if !curie.contains(':') {\n");
        methods.push_str("            if let Some(ref default_prefix) = self.default_prefix {\n");
        if self.config.thread_safe {
            methods.push_str("                if let Some(namespace) = prefixes.get(default_prefix) {\n");
        } else {
            methods.push_str("                if let Some(namespace) = self.prefixes.get(default_prefix) {\n");
        }
        methods.push_str("                    return Ok(format!(\"{}{}\", namespace, curie));\n");
        methods.push_str("                }\n");
        methods.push_str("            }\n");
        methods.push_str("            return Ok(curie.to_string());\n");
        methods.push_str("        }\n");
        methods.push_str("        \n");
        methods.push_str("        let parts: Vec<&str> = curie.splitn(2, ':').collect();\n");
        methods.push_str("        if parts.len() != 2 {\n");
        methods.push_str("            return Err(format!(\"Invalid CURIE format: {}\", curie));\n");
        methods.push_str("        }\n");
        methods.push_str("        \n");
        methods.push_str("        let (prefix, local_name) = (parts[0], parts[1]);\n");
        methods.push_str("        \n");
        if self.config.thread_safe {
            methods.push_str("        if let Some(namespace) = prefixes.get(prefix) {\n");
        } else {
            methods.push_str("        if let Some(namespace) = self.prefixes.get(prefix) {\n");
        }
        methods.push_str("            Ok(format!(\"{}{}\", namespace, local_name))\n");
        methods.push_str("        } else {\n");
        methods.push_str("            Err(format!(\"Unknown prefix: {}\", prefix))\n");
        methods.push_str("        }\n");
        methods.push_str("    }\n\n");
        
        // Contract method
        methods.push_str("    /// Contract a URI to a CURIE if possible\n");
        methods.push_str("    pub fn contract(&self, uri: &str) -> Option<String> {\n");
        
        if self.config.thread_safe {
            methods.push_str("        let namespaces = self.namespaces.read().ok()?;\n");
        }
        
        methods.push_str("        let mut best_match = None;\n");
        methods.push_str("        let mut best_length = 0;\n");
        methods.push_str("        \n");
        
        if self.config.thread_safe {
            methods.push_str("        for (namespace, prefix) in namespaces.iter() {\n");
        } else {
            methods.push_str("        for (namespace, prefix) in &self.namespaces {\n");
        }
        methods.push_str("            if uri.starts_with(namespace) && namespace.len() > best_length {\n");
        methods.push_str("                best_match = Some((namespace, prefix));\n");
        methods.push_str("                best_length = namespace.len();\n");
        methods.push_str("            }\n");
        methods.push_str("        }\n");
        methods.push_str("        \n");
        methods.push_str("        if let Some((namespace, prefix)) = best_match {\n");
        methods.push_str("            let local_name = &uri[namespace.len()..];\n");
        methods.push_str("            Some(format!(\"{}:{}\", prefix, local_name))\n");
        methods.push_str("        } else {\n");
        methods.push_str("            None\n");
        methods.push_str("        }\n");
        methods.push_str("    }\n\n");
        
        // Bind method
        methods.push_str("    /// Bind a new prefix to a namespace\n");
        methods.push_str("    pub fn bind(&mut self, prefix: String, namespace: String) ");
        if self.config.thread_safe {
            methods.push_str("-> Result<(), String> {\n");
            methods.push_str("        let mut prefixes = self.prefixes.write().map_err(|_| \"Lock poisoned\")?;\n");
            methods.push_str("        let mut namespaces = self.namespaces.write().map_err(|_| \"Lock poisoned\")?;\n");
            methods.push_str("        \n");
            methods.push_str("        prefixes.insert(prefix.clone(), namespace.clone());\n");
            methods.push_str("        namespaces.insert(namespace, prefix);\n");
            methods.push_str("        Ok(())\n");
        } else {
            methods.push_str("{\n");
            methods.push_str("        self.prefixes.insert(prefix.clone(), namespace.clone());\n");
            methods.push_str("        self.namespaces.insert(namespace, prefix);\n");
        }
        methods.push_str("    }\n");
        
        methods
    }
    
    /// Generate Java namespace manager
    fn generate_java(&self, schema: &Schema) -> Result<String, LinkMLError> {
        let mut output = String::new();
        
        output.push_str("/**\n");
        output.push_str(" * Namespace manager generated from LinkML schema\n");
        output.push_str(" */\n\n");
        
        output.push_str("package org.linkml.namespace;\n\n");
        
        output.push_str("import java.util.Map;\n");
        output.push_str("import java.util.HashMap;\n");
        output.push_str("import java.util.Optional;\n");
        if self.config.thread_safe {
            output.push_str("import java.util.concurrent.ConcurrentHashMap;\n");
        }
        output.push_str("\n");
        
        // Class definition
        output.push_str(&format!("public class {} {{\n", self.config.class_name));
        
        if self.config.thread_safe {
            output.push_str("    private final Map<String, String> prefixes = new ConcurrentHashMap<>();\n");
            output.push_str("    private final Map<String, String> namespaces = new ConcurrentHashMap<>();\n");
        } else {
            output.push_str("    private final Map<String, String> prefixes = new HashMap<>();\n");
            output.push_str("    private final Map<String, String> namespaces = new HashMap<>();\n");
        }
        output.push_str("    private final String defaultPrefix;\n\n");
        
        // Constructor
        output.push_str(&format!("    public {}() {{\n", self.config.class_name));
        
        // Add schema prefixes
        if let Some(schema_prefixes) = &schema.prefixes {
            for (prefix, expansion) in schema_prefixes {
                output.push_str(&format!(
                    "        prefixes.put(\"{}\", \"{}\");\n",
                    prefix, expansion.prefix_reference
                ));
            }
        }
        
        // Add common prefixes
        output.push_str("        \n");
        output.push_str("        // Common semantic web prefixes\n");
        output.push_str("        prefixes.put(\"rdf\", \"http://www.w3.org/1999/02/22-rdf-syntax-ns#\");\n");
        output.push_str("        prefixes.put(\"rdfs\", \"http://www.w3.org/2000/01/rdf-schema#\");\n");
        output.push_str("        prefixes.put(\"xsd\", \"http://www.w3.org/2001/XMLSchema#\");\n");
        output.push_str("        prefixes.put(\"owl\", \"http://www.w3.org/2002/07/owl#\");\n");
        output.push_str("        \n");
        
        output.push_str("        // Build reverse mapping\n");
        output.push_str("        prefixes.forEach((k, v) -> namespaces.put(v, k));\n");
        output.push_str("        \n");
        
        if let Some(default_prefix) = &schema.default_prefix {
            output.push_str(&format!("        this.defaultPrefix = \"{}\";\n", default_prefix));
        } else {
            output.push_str("        this.defaultPrefix = null;\n");
        }
        output.push_str("    }\n\n");
        
        // Core methods
        output.push_str(&self.generate_java_methods());
        
        output.push_str("}\n");
        
        Ok(output)
    }
    
    /// Generate Java methods
    fn generate_java_methods(&self) -> String {
        let mut methods = String::new();
        
        // Expand method
        methods.push_str("    /**\n");
        methods.push_str("     * Expand a CURIE to a full URI\n");
        methods.push_str("     * @param curie Compact URI (e.g., 'ex:Person')\n");
        methods.push_str("     * @return Expanded URI\n");
        methods.push_str("     * @throws IllegalArgumentException If prefix is not registered\n");
        methods.push_str("     */\n");
        methods.push_str("    public String expand(String curie) {\n");
        methods.push_str("        if (!curie.contains(\":\")) {\n");
        methods.push_str("            if (defaultPrefix != null && prefixes.containsKey(defaultPrefix)) {\n");
        methods.push_str("                return prefixes.get(defaultPrefix) + curie;\n");
        methods.push_str("            }\n");
        methods.push_str("            return curie;\n");
        methods.push_str("        }\n");
        methods.push_str("        \n");
        methods.push_str("        String[] parts = curie.split(\":\", 2);\n");
        methods.push_str("        String prefix = parts[0];\n");
        methods.push_str("        String localName = parts[1];\n");
        methods.push_str("        \n");
        methods.push_str("        if (prefixes.containsKey(prefix)) {\n");
        methods.push_str("            return prefixes.get(prefix) + localName;\n");
        methods.push_str("        }\n");
        methods.push_str("        \n");
        methods.push_str("        throw new IllegalArgumentException(\"Unknown prefix: \" + prefix);\n");
        methods.push_str("    }\n\n");
        
        // Contract method
        methods.push_str("    /**\n");
        methods.push_str("     * Contract a URI to a CURIE if possible\n");
        methods.push_str("     * @param uri Full URI to contract\n");
        methods.push_str("     * @return CURIE if contraction possible\n");
        methods.push_str("     */\n");
        methods.push_str("    public Optional<String> contract(String uri) {\n");
        methods.push_str("        String bestNamespace = null;\n");
        methods.push_str("        int bestLength = 0;\n");
        methods.push_str("        \n");
        methods.push_str("        for (Map.Entry<String, String> entry : namespaces.entrySet()) {\n");
        methods.push_str("            String namespace = entry.getKey();\n");
        methods.push_str("            if (uri.startsWith(namespace) && namespace.length() > bestLength) {\n");
        methods.push_str("                bestNamespace = namespace;\n");
        methods.push_str("                bestLength = namespace.length();\n");
        methods.push_str("            }\n");
        methods.push_str("        }\n");
        methods.push_str("        \n");
        methods.push_str("        if (bestNamespace != null) {\n");
        methods.push_str("            String prefix = namespaces.get(bestNamespace);\n");
        methods.push_str("            String localName = uri.substring(bestNamespace.length());\n");
        methods.push_str("            return Optional.of(prefix + \":\" + localName);\n");
        methods.push_str("        }\n");
        methods.push_str("        \n");
        methods.push_str("        return Optional.empty();\n");
        methods.push_str("    }\n");
        
        methods
    }
    
    /// Generate Go namespace manager
    fn generate_go(&self, schema: &Schema) -> Result<String, LinkMLError> {
        let mut output = String::new();
        
        output.push_str("// Package namespace provides namespace management for LinkML schemas\n");
        output.push_str("package namespace\n\n");
        
        output.push_str("import (\n");
        output.push_str("\t\"fmt\"\n");
        output.push_str("\t\"strings\"\n");
        if self.config.thread_safe {
            output.push_str("\t\"sync\"\n");
        }
        output.push_str(")\n\n");
        
        // Struct definition
        output.push_str("// Manager manages namespace prefixes and URI expansion/contraction\n");
        output.push_str("type Manager struct {\n");
        if self.config.thread_safe {
            output.push_str("\tmu sync.RWMutex\n");
        }
        output.push_str("\tprefixes map[string]string\n");
        output.push_str("\tnamespaces map[string]string\n");
        output.push_str("\tdefaultPrefix string\n");
        output.push_str("}\n\n");
        
        // Constructor
        output.push_str("// NewManager creates a new namespace manager\n");
        output.push_str("func NewManager() *Manager {\n");
        output.push_str("\tm := &Manager{\n");
        output.push_str("\t\tprefixes: make(map[string]string),\n");
        output.push_str("\t\tnamespaces: make(map[string]string),\n");
        output.push_str("\t}\n\n");
        
        // Add schema prefixes
        if let Some(schema_prefixes) = &schema.prefixes {
            for (prefix, expansion) in schema_prefixes {
                output.push_str(&format!(
                    "\tm.prefixes[\"{}\"] = \"{}\"\n",
                    prefix, expansion.prefix_reference
                ));
            }
            output.push_str("\n");
        }
        
        // Add common prefixes
        output.push_str("\t// Common semantic web prefixes\n");
        output.push_str("\tm.prefixes[\"rdf\"] = \"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"\n");
        output.push_str("\tm.prefixes[\"rdfs\"] = \"http://www.w3.org/2000/01/rdf-schema#\"\n");
        output.push_str("\tm.prefixes[\"xsd\"] = \"http://www.w3.org/2001/XMLSchema#\"\n");
        output.push_str("\tm.prefixes[\"owl\"] = \"http://www.w3.org/2002/07/owl#\"\n\n");
        
        output.push_str("\t// Build reverse mapping\n");
        output.push_str("\tfor prefix, namespace := range m.prefixes {\n");
        output.push_str("\t\tm.namespaces[namespace] = prefix\n");
        output.push_str("\t}\n\n");
        
        if let Some(default_prefix) = &schema.default_prefix {
            output.push_str(&format!("\tm.defaultPrefix = \"{}\"\n", default_prefix));
        }
        
        output.push_str("\treturn m\n");
        output.push_str("}\n\n");
        
        // Core methods
        output.push_str(&self.generate_go_methods());
        
        Ok(output)
    }
    
    /// Generate Go methods
    fn generate_go_methods(&self) -> String {
        let mut methods = String::new();
        
        // Expand method
        methods.push_str("// Expand expands a CURIE to a full URI\n");
        methods.push_str("func (m *Manager) Expand(curie string) (string, error) {\n");
        if self.config.thread_safe {
            methods.push_str("\tm.mu.RLock()\n");
            methods.push_str("\tdefer m.mu.RUnlock()\n\n");
        }
        
        methods.push_str("\tif !strings.Contains(curie, \":\") {\n");
        methods.push_str("\t\tif m.defaultPrefix != \"\" {\n");
        methods.push_str("\t\t\tif namespace, ok := m.prefixes[m.defaultPrefix]; ok {\n");
        methods.push_str("\t\t\t\treturn namespace + curie, nil\n");
        methods.push_str("\t\t\t}\n");
        methods.push_str("\t\t}\n");
        methods.push_str("\t\treturn curie, nil\n");
        methods.push_str("\t}\n\n");
        
        methods.push_str("\tparts := strings.SplitN(curie, \":\", 2)\n");
        methods.push_str("\tif len(parts) != 2 {\n");
        methods.push_str("\t\treturn \"\", fmt.Errorf(\"invalid CURIE format: %s\", curie)\n");
        methods.push_str("\t}\n\n");
        
        methods.push_str("\tprefix, localName := parts[0], parts[1]\n\n");
        
        methods.push_str("\tif namespace, ok := m.prefixes[prefix]; ok {\n");
        methods.push_str("\t\treturn namespace + localName, nil\n");
        methods.push_str("\t}\n\n");
        
        methods.push_str("\treturn \"\", fmt.Errorf(\"unknown prefix: %s\", prefix)\n");
        methods.push_str("}\n\n");
        
        // Contract method
        methods.push_str("// Contract contracts a URI to a CURIE if possible\n");
        methods.push_str("func (m *Manager) Contract(uri string) string {\n");
        if self.config.thread_safe {
            methods.push_str("\tm.mu.RLock()\n");
            methods.push_str("\tdefer m.mu.RUnlock()\n\n");
        }
        
        methods.push_str("\tvar bestNamespace string\n");
        methods.push_str("\tvar bestPrefix string\n");
        methods.push_str("\tbestLength := 0\n\n");
        
        methods.push_str("\tfor namespace, prefix := range m.namespaces {\n");
        methods.push_str("\t\tif strings.HasPrefix(uri, namespace) && len(namespace) > bestLength {\n");
        methods.push_str("\t\t\tbestNamespace = namespace\n");
        methods.push_str("\t\t\tbestPrefix = prefix\n");
        methods.push_str("\t\t\tbestLength = len(namespace)\n");
        methods.push_str("\t\t}\n");
        methods.push_str("\t}\n\n");
        
        methods.push_str("\tif bestNamespace != \"\" {\n");
        methods.push_str("\t\tlocalName := uri[len(bestNamespace):]\n");
        methods.push_str("\t\treturn fmt.Sprintf(\"%s:%s\", bestPrefix, localName)\n");
        methods.push_str("\t}\n\n");
        
        methods.push_str("\treturn uri\n");
        methods.push_str("}\n");
        
        methods
    }
}

impl Generator for NamespaceManagerGenerator {
    fn generate(&self, schema: &Schema) -> Result<String, LinkMLError> {
        self.generate_manager(schema)
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
    
    fn get_default_filename(&self) -> &str {
        "namespace_manager"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::schema::SchemaDefinition;
    
    #[test]
    fn test_namespace_manager_generation() {
        let mut schema = SchemaDefinition::default();
        schema.name = Some("TestSchema".to_string());
        
        // Add prefixes
        let mut prefixes = HashMap::new();
        prefixes.insert(
            "ex".to_string(),
            Prefix {
                prefix_prefix: "ex".to_string(),
                prefix_reference: "https://example.com/".to_string(),
            },
        );
        schema.prefixes = Some(prefixes);
        schema.default_prefix = Some("ex".to_string());
        
        // Test Python generation
        let config = NamespaceManagerGeneratorConfig::default();
        let generator = NamespaceManagerGenerator::new(config);
        let result = generator.generate(&Schema(schema)).expect("should generate namespace manager");
        
        assert!(result.contains("class NamespaceManager:"));
        assert!(result.contains("def expand("));
        assert!(result.contains("def contract("));
    }
}