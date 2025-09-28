//! IDE integration support for LinkML
//!
//! This module provides IDE features including:
//! - Language Server Protocol (LSP) implementation
//! - Syntax highlighting configurations
//! - Code completion providers
//! - Diagnostics and error reporting
//! - Go to definition
//! - Find references
//! - Hover information
//! - Code actions and quick fixes

use linkml_core::types::SchemaDefinition;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// Language server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageServerConfig {
    /// Enable diagnostics
    pub diagnostics: bool,
    /// Enable code completion
    pub completion: bool,
    /// Enable hover information
    pub hover: bool,
    /// Enable go to definition
    pub goto_definition: bool,
    /// Enable find references
    pub find_references: bool,
    /// Enable code actions
    pub code_actions: bool,
    /// Maximum number of diagnostics
    pub max_diagnostics: usize,
    /// Validation on save
    pub validate_on_save: bool,
    /// Validation on change
    pub validate_on_change: bool,
}

impl Default for LanguageServerConfig {
    fn default() -> Self {
        Self {
            diagnostics: true,
            completion: true,
            hover: true,
            goto_definition: true,
            find_references: true,
            code_actions: true,
            max_diagnostics: 100,
            validate_on_save: true,
            validate_on_change: false,
        }
    }
}

/// Syntax highlighting configuration for various editors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxHighlighting {
    /// `TextMate` grammar
    pub textmate_grammar: TextMateGrammar,
    /// Tree-sitter grammar
    pub tree_sitter_grammar: String,
    /// Vim syntax file
    pub vim_syntax: String,
    /// Emacs mode
    pub emacs_mode: String,
}

/// `TextMate` grammar for VS Code and other editors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextMateGrammar {
    /// Grammar name
    pub name: String,
    /// Scope name
    pub scope_name: String,
    /// File types
    pub file_types: Vec<String>,
    /// Patterns
    pub patterns: Vec<GrammarPattern>,
    /// Repository of reusable patterns
    pub repository: HashMap<String, GrammarPattern>,
}

/// Grammar pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrammarPattern {
    /// Pattern name
    pub name: Option<String>,
    /// Match regex
    pub r#match: Option<String>,
    /// Begin regex
    pub begin: Option<String>,
    /// End regex
    pub end: Option<String>,
    /// Captures
    pub captures: Option<HashMap<String, Capture>>,
    /// Include other patterns
    pub include: Option<String>,
    /// Sub-patterns
    pub patterns: Option<Vec<GrammarPattern>>,
}

/// Capture group
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capture {
    /// Scope name
    pub name: String,
}

/// Generate `TextMate` grammar for `LinkML`
#[must_use]
pub fn generate_textmate_grammar() -> TextMateGrammar {
    let mut repository = HashMap::new();

    // Keywords
    repository.insert("keywords".to_string(), GrammarPattern {
        name: Some("keyword.control.linkml".to_string()),
        r#match: Some(r"\b(classes|slots|types|enums|subsets|prefixes|imports|id|name|title|description|is_a|mixins|abstract|tree_root|range|required|multivalued|pattern|minimum_value|maximum_value|enum_range|permissible_values|typeof|uri|base|prefix_prefix|prefix_reference)\b".to_string()),
        begin: None,
        end: None,
        captures: None,
        include: None,
        patterns: None});

    // Strings
    repository.insert(
        "strings".to_string(),
        GrammarPattern {
            name: Some("string.quoted.double.linkml".to_string()),
            begin: Some("\"".to_string()),
            end: Some("\"".to_string()),
            r#match: None,
            captures: None,
            include: None,
            patterns: Some(vec![GrammarPattern {
                name: Some("constant.character.escape.linkml".to_string()),
                r#match: Some(
                    r#"\\["\
rt]"#
                        .to_string(),
                ),
                begin: None,
                end: None,
                captures: None,
                include: None,
                patterns: None,
            }]),
        },
    );

    // Comments
    repository.insert(
        "comments".to_string(),
        GrammarPattern {
            name: Some("comment.line.number-sign.yaml".to_string()),
            r#match: Some(r"#.*$".to_string()),
            begin: None,
            end: None,
            captures: None,
            include: None,
            patterns: None,
        },
    );

    // Numbers
    repository.insert(
        "numbers".to_string(),
        GrammarPattern {
            name: Some("constant.numeric.yaml".to_string()),
            r#match: Some(r"\b\d+(\.\d+)?\b".to_string()),
            begin: None,
            end: None,
            captures: None,
            include: None,
            patterns: None,
        },
    );

    // Booleans
    repository.insert(
        "booleans".to_string(),
        GrammarPattern {
            name: Some("constant.language.boolean.yaml".to_string()),
            r#match: Some(r"\b(true|false)\b".to_string()),
            begin: None,
            end: None,
            captures: None,
            include: None,
            patterns: None,
        },
    );

    TextMateGrammar {
        name: "LinkML".to_string(),
        scope_name: "source.linkml".to_string(),
        file_types: vec!["yaml".to_string(), "yml".to_string(), "linkml".to_string()],
        patterns: vec![
            GrammarPattern {
                include: Some("#keywords".to_string()),
                name: None,
                r#match: None,
                begin: None,
                end: None,
                captures: None,
                patterns: None,
            },
            GrammarPattern {
                include: Some("#strings".to_string()),
                name: None,
                r#match: None,
                begin: None,
                end: None,
                captures: None,
                patterns: None,
            },
            GrammarPattern {
                include: Some("#comments".to_string()),
                name: None,
                r#match: None,
                begin: None,
                end: None,
                captures: None,
                patterns: None,
            },
            GrammarPattern {
                include: Some("#numbers".to_string()),
                name: None,
                r#match: None,
                begin: None,
                end: None,
                captures: None,
                patterns: None,
            },
            GrammarPattern {
                include: Some("#booleans".to_string()),
                name: None,
                r#match: None,
                begin: None,
                end: None,
                captures: None,
                patterns: None,
            },
        ],
        repository,
    }
}

/// Generate Tree-sitter grammar for `LinkML`
#[must_use]
pub fn generate_tree_sitter_grammar() -> String {
    r#"
module.exports = grammar({
  name: 'linkml',

  rules: {
    source_file: $ => repeat($._definition),

    definition: $ => choice(
      $.class_definition,
      $.slot_definition,
      $.type_definition,
      $.enum_definition
    ),

    class_definition: $ => seq(
      'class:',
      $.identifier,
      ':',
      $._class_body
    ),

    class_body: $ => repeat1(
      choice(
        $.is_a,
        $.slots,
        $.attributes,
        $.description
      )
    ),

    slot_definition: $ => seq(
      'slot:',
      $.identifier,
      ':',
      $._slot_body
    ),

    slot_body: $ => repeat1(
      choice(
        $.range,
        $.required,
        $.multivalued,
        $.pattern,
        $.description
      )
    ),

    is_a: $ => seq('is_a:', $.identifier),
    slots: $ => seq('slots:', $.slot_list),
    range: $ => seq('range:', $.type),
    required: $ => seq('required:', $.boolean),
    multivalued: $ => seq('multivalued:', $.boolean),
    pattern: $ => seq('pattern:', $.string),
    description: $ => seq('description:', $.string),

    slot_list: $ => repeat1(seq('-', $.identifier)),

    identifier: $ => /[a-zA-Z_][a-zA-Z0-9_]*/,
    type: $ => /[a-zA-Z_][a-zA-Z0-9_]*/,
    string: $ => /"[^"]*"/,
    boolean: $ => choice('true', 'false'),

    comment: $ => /#.*/
  }
});
"#
    .to_string()
}

/// Generate Vim syntax file
#[must_use]
pub fn generate_vim_syntax() -> String {
    r#"" Vim syntax file for LinkML
" Language: LinkML
" Maintainer: LinkML Service

if exists("b:current_syntax")
  finish
endif

" Keywords
syn keyword linkmlKeyword classes slots types enums subsets prefixes imports
syn keyword linkmlKeyword id name title description is_a mixins abstract
syn keyword linkmlKeyword tree_root range required multivalued pattern
syn keyword linkmlKeyword minimum_value maximum_value enum_range
syn keyword linkmlKeyword permissible_values typeof uri base
syn keyword linkmlKeyword prefix_prefix prefix_reference

" Types
syn keyword linkmlType string integer float boolean date datetime uri

" Booleans
syn keyword linkmlBoolean true false

" Strings
syn region linkmlString start=/"/ end=/"/ contains=linkmlEscape
syn match linkmlEscape /\\["\\/nrt]/ contained

" Comments
syn match linkmlComment /#.*/

" Numbers
syn match linkmlNumber /\<\d\+\(\.\d\+\)\?\>/

" Highlighting
hi def link linkmlKeyword Keyword
hi def link linkmlType Type
hi def link linkmlBoolean Boolean
hi def link linkmlString String
hi def link linkmlEscape SpecialChar
hi def link linkmlComment Comment
hi def link linkmlNumber Number

let b:current_syntax = "linkml"
"#
    .to_string()
}

/// Generate Emacs mode
#[must_use]
pub fn generate_emacs_mode() -> String {
    r##";;; linkml-mode.el --- Major mode for editing LinkML schemas

(defvar linkml-mode-hook nil)

(defvar linkml-mode-map
  (let ((map (make-keymap)))
    (define-key map "\C-c\C-v" 'linkml-validate)
    map)
  "Keymap for LinkML major mode")

(defconst linkml-font-lock-keywords
  '(("\\<\\(classes\\|slots\\|types\\|enums\\|subsets\\|prefixes\\|imports\\)\\>" . font-lock-keyword-face)
    ("\\<\\(id\\|name\\|title\\|description\\|is_a\\|mixins\\|abstract\\)\\>" . font-lock-keyword-face)
    ("\\<\\(tree_root\\|range\\|required\\|multivalued\\|pattern\\)\\>" . font-lock-keyword-face)
    ("\\<\\(minimum_value\\|maximum_value\\|enum_range\\|permissible_values\\)\\>" . font-lock-keyword-face)
    ("\\<\\(typeof\\|uri\\|base\\|prefix_prefix\\|prefix_reference\\)\\>" . font-lock-keyword-face)
    ("\\<\\(string\\|integer\\|float\\|boolean\\|date\\|datetime\\|uri\\)\\>" . font-lock-type-face)
    ("\\<\\(true\\|false\\)\\>" . font-lock-constant-face)
    ("#.*" . font-lock-comment-face))
  "Highlighting expressions for LinkML mode")

(defun linkml-mode ()
  "Major mode for editing LinkML schema files"
  (interactive)
  (kill-all-local-variables)
  (use-local-map linkml-mode-map)
  (set (make-local-variable 'font-lock-defaults) '(linkml-font-lock-keywords))
  (setq major-mode 'linkml-mode)
  (setq mode-name "LinkML")
  (run-hooks 'linkml-mode-hook))

(provide 'linkml-mode)
;;; linkml-mode.el ends here
"##.to_string()
}

/// Code completion provider
#[derive(Debug, Clone)]
pub struct CompletionProvider {
    /// Schema context
    schema: Option<Arc<SchemaDefinition>>,
    /// Keywords
    keywords: Vec<CompletionItem>,
    /// Types
    types: Vec<CompletionItem>,
}

/// Completion item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionItem {
    /// Label
    pub label: String,
    /// Kind
    pub kind: CompletionKind,
    /// Detail
    pub detail: Option<String>,
    /// Documentation
    pub documentation: Option<String>,
    /// Insert text
    pub insert_text: Option<String>,
}

/// Completion item kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompletionKind {
    /// Keyword
    Keyword,
    /// Type
    Type,
    /// Class
    Class,
    /// Slot
    Slot,
    /// Enum
    Enum,
    /// Value
    Value,
    /// Snippet
    Snippet,
}

impl Default for CompletionProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl CompletionProvider {
    /// Create new completion provider
    #[must_use]
    pub fn new() -> Self {
        let keywords = vec![
            CompletionItem {
                label: "classes".to_string(),
                kind: CompletionKind::Keyword,
                detail: Some("Define classes".to_string()),
                documentation: Some("Define the classes in the schema".to_string()),
                insert_text: Some(
                    "classes:
  "
                    .to_string(),
                ),
            },
            CompletionItem {
                label: "slots".to_string(),
                kind: CompletionKind::Keyword,
                detail: Some("Define slots".to_string()),
                documentation: Some("Define the slots in the schema".to_string()),
                insert_text: Some(
                    "slots:
  "
                    .to_string(),
                ),
            },
            CompletionItem {
                label: "is_a".to_string(),
                kind: CompletionKind::Keyword,
                detail: Some("Parent class".to_string()),
                documentation: Some("Specify the parent class for inheritance".to_string()),
                insert_text: Some("is_a: ".to_string()),
            },
            CompletionItem {
                label: "required".to_string(),
                kind: CompletionKind::Keyword,
                detail: Some("Required field".to_string()),
                documentation: Some("Specify if this field is required".to_string()),
                insert_text: Some("required: true".to_string()),
            },
        ];

        let types = vec![
            CompletionItem {
                label: "string".to_string(),
                kind: CompletionKind::Type,
                detail: Some("String type".to_string()),
                documentation: Some("A sequence of characters".to_string()),
                insert_text: None,
            },
            CompletionItem {
                label: "integer".to_string(),
                kind: CompletionKind::Type,
                detail: Some("Integer type".to_string()),
                documentation: Some("A whole number".to_string()),
                insert_text: None,
            },
            CompletionItem {
                label: "float".to_string(),
                kind: CompletionKind::Type,
                detail: Some("Float type".to_string()),
                documentation: Some("A floating point number".to_string()),
                insert_text: None,
            },
            CompletionItem {
                label: "boolean".to_string(),
                kind: CompletionKind::Type,
                detail: Some("Boolean type".to_string()),
                documentation: Some("True or false value".to_string()),
                insert_text: None,
            },
        ];

        Self {
            schema: None,
            keywords,
            types,
        }
    }

    /// Set schema context
    pub fn set_schema(&mut self, schema: Arc<SchemaDefinition>) {
        self.schema = Some(schema);
    }

    /// Get completions at position
    #[must_use]
    pub fn get_completions(&self, context: &CompletionContext) -> Vec<CompletionItem> {
        let mut completions = Vec::new();

        // Add keywords
        if context.is_top_level {
            completions.extend(self.keywords.clone());
        }

        // Add types
        if context.expecting_type {
            completions.extend(self.types.clone());
        }

        // Add schema-specific completions
        if let Some(schema) = &self.schema {
            if context.expecting_class {
                for (name, class) in &schema.classes {
                    completions.push(CompletionItem {
                        label: name.clone(),
                        kind: CompletionKind::Class,
                        detail: class.description.clone(),
                        documentation: None,
                        insert_text: None,
                    });
                }
            }

            if context.expecting_slot {
                for (name, slot) in &schema.slots {
                    completions.push(CompletionItem {
                        label: name.clone(),
                        kind: CompletionKind::Slot,
                        detail: slot.description.clone(),
                        documentation: None,
                        insert_text: None,
                    });
                }
            }
        }

        completions
    }
}

/// Completion context
#[derive(Debug, Clone)]
pub struct CompletionContext {
    /// Is at top level
    pub is_top_level: bool,
    /// Expecting a type
    pub expecting_type: bool,
    /// Expecting a class name
    pub expecting_class: bool,
    /// Expecting a slot name
    pub expecting_slot: bool,
    /// Current line
    pub line: String,
    /// Cursor position
    pub position: usize,
}

/// Diagnostic provider
pub struct DiagnosticProvider<S>
where
    S: linkml_core::traits::LinkMLService,
{
    /// `LinkML` service
    service: Arc<S>,
    /// Current diagnostics
    diagnostics: Arc<RwLock<Vec<Diagnostic>>>,
}

/// Diagnostic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Range in document
    pub range: Range,
    /// Severity
    pub severity: DiagnosticSeverity,
    /// Message
    pub message: String,
    /// Source
    pub source: String,
    /// Code
    pub code: Option<String>,
}

/// Diagnostic severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticSeverity {
    /// Error
    Error,
    /// Warning
    Warning,
    /// Information
    Information,
    /// Hint
    Hint,
}

/// Range in document
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Range {
    /// Start position
    pub start: Position,
    /// End position
    pub end: Position,
}

/// Position in document
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    /// Line (0-based)
    pub line: u32,
    /// Character (0-based)
    pub character: u32,
}

impl<S> DiagnosticProvider<S>
where
    S: linkml_core::traits::LinkMLService,
{
    /// Create new diagnostic provider
    pub fn new(service: Arc<S>) -> Self {
        Self {
            service,
            diagnostics: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Validate document
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub async fn validate(&self, content: &str) -> linkml_core::error::Result<Vec<Diagnostic>> {
        let mut diagnostics = Vec::new();

        // Try to parse as YAML
        match serde_yaml::from_str::<Value>(content) {
            Ok(_) => {
                // Parse as LinkML schema
                match self
                    .service
                    .load_schema_str(content, linkml_core::traits::SchemaFormat::Yaml)
                    .await
                {
                    Ok(_schema) => {
                        // Schema is valid
                    }
                    Err(e) => {
                        diagnostics.push(Diagnostic {
                            range: Range {
                                start: Position {
                                    line: 0,
                                    character: 0,
                                },
                                end: Position {
                                    line: 0,
                                    character: 0,
                                },
                            },
                            severity: DiagnosticSeverity::Error,
                            message: format!("Invalid LinkML schema: {e}"),
                            source: "linkml".to_string(),
                            code: Some("E001".to_string()),
                        });
                    }
                }
            }
            Err(e) => {
                // YAML parse error
                diagnostics.push(Diagnostic {
                    range: Range {
                        start: Position {
                            line: 0,
                            character: 0,
                        },
                        end: Position {
                            line: 0,
                            character: 0,
                        },
                    },
                    severity: DiagnosticSeverity::Error,
                    message: format!("YAML parse error: {e}"),
                    source: "yaml".to_string(),
                    code: None,
                });
            }
        }

        self.diagnostics.write().clone_from(&diagnostics);
        Ok(diagnostics)
    }

    /// Get current diagnostics
    #[must_use]
    pub fn get_diagnostics(&self) -> Vec<Diagnostic> {
        self.diagnostics.read().clone()
    }
}

/// VS Code extension configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VSCodeExtension {
    /// Extension name
    pub name: String,
    /// Display name
    pub display_name: String,
    /// Description
    pub description: String,
    /// Version
    pub version: String,
    /// Publisher
    pub publisher: String,
    /// Categories
    pub categories: Vec<String>,
    /// Keywords
    pub keywords: Vec<String>,
    /// Contributes
    pub contributes: VSCodeContributes,
}

/// VS Code contributions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VSCodeContributes {
    /// Languages
    pub languages: Vec<LanguageConfiguration>,
    /// Grammars
    pub grammars: Vec<GrammarConfiguration>,
    /// Commands
    pub commands: Vec<CommandConfiguration>,
    /// Configuration
    pub configuration: ConfigurationSchema,
}

/// Language configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageConfiguration {
    /// Language ID
    pub id: String,
    /// Aliases
    pub aliases: Vec<String>,
    /// Extensions
    pub extensions: Vec<String>,
    /// Configuration file
    pub configuration: String,
}

/// Grammar configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrammarConfiguration {
    /// Language
    pub language: String,
    /// Scope name
    pub scope_name: String,
    /// Path to grammar file
    pub path: String,
}

/// Command configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandConfiguration {
    /// Command ID
    pub command: String,
    /// Title
    pub title: String,
    /// Category
    pub category: Option<String>,
}

/// Configuration schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurationSchema {
    /// Title
    pub title: String,
    /// Properties
    pub properties: HashMap<String, ConfigProperty>,
}

/// Configuration property
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigProperty {
    /// Type
    pub r#type: String,
    /// Default value
    pub default: Value,
    /// Description
    pub description: String,
}

/// Generate VS Code extension configuration
#[must_use]
pub fn generate_vscode_extension() -> VSCodeExtension {
    let mut properties = HashMap::new();

    properties.insert(
        "linkml.validation.onSave".to_string(),
        ConfigProperty {
            r#type: "boolean".to_string(),
            default: Value::Bool(true),
            description: "Enable validation on save".to_string(),
        },
    );

    properties.insert(
        "linkml.validation.onChange".to_string(),
        ConfigProperty {
            r#type: "boolean".to_string(),
            default: Value::Bool(false),
            description: "Enable validation on change".to_string(),
        },
    );

    properties.insert(
        "linkml.completion.enabled".to_string(),
        ConfigProperty {
            r#type: "boolean".to_string(),
            default: Value::Bool(true),
            description: "Enable code completion".to_string(),
        },
    );

    VSCodeExtension {
        name: "linkml".to_string(),
        display_name: "LinkML Language Support".to_string(),
        description: "Language support for LinkML schema files".to_string(),
        version: "0.1.0".to_string(),
        publisher: "rootreal".to_string(),
        categories: vec![
            "Programming Languages".to_string(),
            "Linters".to_string(),
            "Snippets".to_string(),
        ],
        keywords: vec![
            "linkml".to_string(),
            "schema".to_string(),
            "validation".to_string(),
            "yaml".to_string(),
        ],
        contributes: VSCodeContributes {
            languages: vec![LanguageConfiguration {
                id: "linkml".to_string(),
                aliases: vec!["LinkML".to_string(), "linkml".to_string()],
                extensions: vec![
                    ".linkml".to_string(),
                    ".yaml".to_string(),
                    ".yml".to_string(),
                ],
                configuration: "./language-configuration.json".to_string(),
            }],
            grammars: vec![GrammarConfiguration {
                language: "linkml".to_string(),
                scope_name: "source.linkml".to_string(),
                path: "./syntaxes/linkml.tmLanguage.json".to_string(),
            }],
            commands: vec![
                CommandConfiguration {
                    command: "linkml.validate".to_string(),
                    title: "Validate LinkML Schema".to_string(),
                    category: Some("LinkML".to_string()),
                },
                CommandConfiguration {
                    command: "linkml.format".to_string(),
                    title: "Format LinkML Schema".to_string(),
                    category: Some("LinkML".to_string()),
                },
            ],
            configuration: ConfigurationSchema {
                title: "LinkML".to_string(),
                properties,
            },
        },
    }
}

/// Generate package.json for VS Code extension
#[must_use]
pub fn generate_vscode_package_json(extension: &VSCodeExtension) -> String {
    serde_json::to_string_pretty(extension).unwrap_or_default()
}

/// IntelliJ/IDEA plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelliJPlugin {
    /// Plugin ID
    pub id: String,
    /// Name
    pub name: String,
    /// Version
    pub version: String,
    /// Vendor
    pub vendor: String,
    /// Description
    pub description: String,
    /// Dependencies
    pub dependencies: Vec<String>,
}

/// Generate `IntelliJ` plugin configuration
#[must_use]
pub fn generate_intellij_plugin() -> IntelliJPlugin {
    IntelliJPlugin {
        id: "com.rootreal.linkml".to_string(),
        name: "LinkML Support".to_string(),
        version: "0.1.0".to_string(),
        vendor: "RootReal".to_string(),
        description: "Support for LinkML schema files".to_string(),
        dependencies: vec![
            "com.intellij.modules.lang".to_string(),
            "org.jetbrains.plugins.yaml".to_string(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completion_provider() {
        let provider = CompletionProvider::new();

        let context = CompletionContext {
            is_top_level: true,
            expecting_type: false,
            expecting_class: false,
            expecting_slot: false,
            line: String::new(),
            position: 0,
        };

        let completions = provider.get_completions(&context);
        assert!(!completions.is_empty());
        assert!(completions.iter().any(|c| c.label == "classes"));
    }

    #[test]
    fn test_textmate_grammar() {
        let grammar = generate_textmate_grammar();
        assert_eq!(grammar.name, "LinkML");
        assert!(!grammar.patterns.is_empty());
        assert!(!grammar.repository.is_empty());
    }
}
