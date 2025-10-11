//! GraphQL schema generation implementation for `LinkML` schemas

use super::options::{GeneratorOptions, IndentStyle};
use super::traits::{CodeFormatter, Generator, GeneratorResult};
use linkml_core::prelude::*;
use std::collections::HashSet;
use std::fmt::Write;

/// GraphQL schema generator for `LinkML` schemas
pub struct GraphQLGenerator {
    /// Generator name
    name: String,
    /// Generator options
    options: super::traits::GeneratorOptions,
}

impl GraphQLGenerator {
    /// Convert `fmt::Error` to `GeneratorError`
    fn fmt_error_to_generator_error(e: std::fmt::Error) -> super::traits::GeneratorError {
        super::traits::GeneratorError::Io(std::io::Error::other(e))
    }

    /// Create a new GraphQL generator
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: "graphql".to_string(),
            options: super::traits::GeneratorOptions::default(),
        }
    }

    /// Create generator with options
    #[must_use]
    pub fn with_options(options: super::traits::GeneratorOptions) -> Self {
        let mut generator = Self::new();
        generator.options = options;
        generator
    }

    /// Generate GraphQL type for a class
    fn generate_class_graphql(
        &self,
        class_name: &str,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
        indent: &IndentStyle,
    ) -> GeneratorResult<String> {
        let mut output = String::new();

        // Documentation
        if options.include_docs
            && let Some(desc) = &class.description
        {
            writeln!(
                &mut output,
                "\"\"\"{desc}
\"\"\""
            )
            .map_err(Self::fmt_error_to_generator_error)?;
        }

        // Type definition
        let type_name = self.convert_identifier(class_name);
        let type_keyword = if class.abstract_ == Some(true) {
            "interface"
        } else {
            "type"
        };

        write!(&mut output, "{type_keyword} {type_name}")
            .map_err(Self::fmt_error_to_generator_error)?;

        // Implements interfaces
        let interfaces = self.collect_interfaces(class, schema);
        if !interfaces.is_empty() {
            write!(&mut output, " implements {}", interfaces.join(" & "))
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(&mut output, " {{").map_err(Self::fmt_error_to_generator_error)?;

        // Generate fields
        self.generate_fields(&mut output, class, schema, options, indent)?;

        writeln!(&mut output, "}}").map_err(Self::fmt_error_to_generator_error)?;

        Ok(output)
    }

    /// Generate fields for a type
    fn generate_fields(
        &self,
        output: &mut String,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
        indent: &IndentStyle,
    ) -> GeneratorResult<()> {
        // Add ID field if this is a root type
        if class.tree_root == Some(true)
            || options
                .get_custom("add_id_field")
                .map(std::string::String::as_str)
                == Some("true")
        {
            writeln!(output, "{}id: ID!", indent.single())
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        // Collect all slots including inherited ones
        let slots = self.collect_all_slots(class, schema)?;

        for slot_name in &slots {
            if let Some(slot) = schema.slots.get(slot_name) {
                // Documentation
                if options.include_docs
                    && let Some(desc) = &slot.description
                {
                    writeln!(output, "{}\"\"\"{}\"\"\"", indent.single(), desc)
                        .map_err(Self::fmt_error_to_generator_error)?;
                }

                // Field definition
                let field_name = Self::convert_field_name(slot_name);
                let field_type = self.get_graphql_type(slot, schema);
                let nullable = if slot.required == Some(true) { "!" } else { "" };

                writeln!(
                    output,
                    "{}{}: {}{}",
                    indent.single(),
                    field_name,
                    field_type,
                    nullable
                )
                .map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        Ok(())
    }

    /// Generate GraphQL enum
    fn generate_enum(
        &self,
        enum_name: &str,
        enum_def: &EnumDefinition,
        options: &GeneratorOptions,
        indent: &IndentStyle,
    ) -> GeneratorResult<String> {
        let mut output = String::new();

        // Documentation
        if options.include_docs
            && let Some(desc) = &enum_def.description
        {
            writeln!(
                &mut output,
                "\"\"\"{desc}
\"\"\""
            )
            .map_err(Self::fmt_error_to_generator_error)?;
        }

        // Enum definition
        let enum_type_name = self.convert_identifier(enum_name);
        writeln!(&mut output, "enum {enum_type_name} {{")
            .map_err(Self::fmt_error_to_generator_error)?;

        // Enum values
        if !enum_def.permissible_values.is_empty() {
            for value_def in &enum_def.permissible_values {
                match value_def {
                    PermissibleValue::Simple(text) => {
                        let value_name = Self::convert_enum_value(text);
                        writeln!(&mut output, "{}{}", indent.single(), value_name)
                            .map_err(Self::fmt_error_to_generator_error)?;
                    }
                    PermissibleValue::Complex {
                        text, description, ..
                    } => {
                        if options.include_docs
                            && let Some(desc) = description
                        {
                            writeln!(&mut output, "{}\"\"\"{}\"\"\"", indent.single(), desc)
                                .map_err(Self::fmt_error_to_generator_error)?;
                        }
                        let value_name = Self::convert_enum_value(text);
                        writeln!(&mut output, "{}{}", indent.single(), value_name)
                            .map_err(Self::fmt_error_to_generator_error)?;
                    }
                }
            }
        }

        writeln!(&mut output, "}}").map_err(Self::fmt_error_to_generator_error)?;

        Ok(output)
    }

    /// Generate input types for mutations
    fn generate_input_types(
        &self,
        schema: &SchemaDefinition,
        _options: &GeneratorOptions,
        indent: &IndentStyle,
    ) -> GeneratorResult<String> {
        let mut output = String::new();

        if !schema.classes.is_empty() {
            writeln!(
                &mut output,
                "
# Input Types for Mutations"
            )
            .map_err(Self::fmt_error_to_generator_error)?;

            for (class_name, class) in &schema.classes {
                // Skip abstract classes for input types
                if class.abstract_ == Some(true) {
                    continue;
                }

                let type_name = self.convert_identifier(class_name);

                // Create input type
                writeln!(
                    &mut output,
                    "
input {type_name}Input {{"
                )
                .map_err(Self::fmt_error_to_generator_error)?;

                // Generate input fields (excluding ID for create operations)
                let slots = self.collect_all_slots(class, schema)?;
                for slot_name in &slots {
                    if let Some(slot) = schema.slots.get(slot_name) {
                        let field_name = Self::convert_field_name(slot_name);
                        let field_type = self.get_graphql_type(slot, schema);

                        // Make all fields optional in input types
                        writeln!(output, "{}{}: {}", indent.single(), field_name, field_type)
                            .map_err(Self::fmt_error_to_generator_error)?;
                    }
                }

                writeln!(&mut output, "}}").map_err(Self::fmt_error_to_generator_error)?;

                // Create update input type (with ID)
                writeln!(
                    &mut output,
                    "
input {type_name}UpdateInput {{"
                )
                .map_err(Self::fmt_error_to_generator_error)?;
                writeln!(&mut output, "{}id: ID!", indent.single())
                    .map_err(Self::fmt_error_to_generator_error)?;

                for slot_name in &slots {
                    if let Some(slot) = schema.slots.get(slot_name) {
                        let field_name = Self::convert_field_name(slot_name);
                        let field_type = self.get_graphql_type(slot, schema);

                        writeln!(output, "{}{}: {}", indent.single(), field_name, field_type)
                            .map_err(Self::fmt_error_to_generator_error)?;
                    }
                }

                writeln!(&mut output, "}}").map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        Ok(output)
    }

    /// Generate root Query type
    fn generate_query_type(
        &self,
        schema: &SchemaDefinition,
        indent: &IndentStyle,
    ) -> GeneratorResult<String> {
        let mut output = String::new();

        writeln!(
            &mut output,
            "
type Query {{"
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        for (class_name, class) in &schema.classes {
            // Skip abstract classes
            if class.abstract_ == Some(true) {
                continue;
            }

            let type_name = self.convert_identifier(class_name);
            let field_name = Self::to_camel_case(&type_name);
            let plural_field = Self::pluralize(&field_name);

            // Single item query
            writeln!(
                &mut output,
                "{}{}(id: ID!): {}",
                indent.single(),
                field_name,
                type_name
            )
            .map_err(Self::fmt_error_to_generator_error)?;

            // List query with pagination
            writeln!(
                &mut output,
                "{}{}(first: Int, after: String, filter: {}Filter): {}Connection!",
                indent.single(),
                plural_field,
                type_name,
                type_name
            )
            .map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(&mut output, "}}").map_err(Self::fmt_error_to_generator_error)?;

        Ok(output)
    }

    /// Generate root Mutation type
    fn generate_mutation_type(
        &self,
        schema: &SchemaDefinition,
        indent: &IndentStyle,
    ) -> GeneratorResult<String> {
        let mut output = String::new();

        writeln!(
            &mut output,
            "
type Mutation {{"
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        for (class_name, class) in &schema.classes {
            // Skip abstract classes
            if class.abstract_ == Some(true) {
                continue;
            }

            let type_name = self.convert_identifier(class_name);
            let _field_name = Self::to_camel_case(&type_name);

            // Create mutation
            writeln!(
                &mut output,
                "{}create{}(input: {}Input!): {}!",
                indent.single(),
                type_name,
                type_name,
                type_name
            )
            .map_err(Self::fmt_error_to_generator_error)?;

            // Update mutation
            writeln!(
                &mut output,
                "{}update{}(input: {}UpdateInput!): {}!",
                indent.single(),
                type_name,
                type_name,
                type_name
            )
            .map_err(Self::fmt_error_to_generator_error)?;

            // Delete mutation
            writeln!(
                &mut output,
                "{}delete{}(id: ID!): Boolean!",
                indent.single(),
                type_name
            )
            .map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(&mut output, "}}").map_err(Self::fmt_error_to_generator_error)?;

        Ok(output)
    }

    /// Generate connection types for pagination
    fn generate_connection_types(
        &self,
        schema: &SchemaDefinition,
        _indent: &IndentStyle,
    ) -> GeneratorResult<String> {
        let mut output = String::new();

        writeln!(
            &mut output,
            "
# Connection Types for Pagination"
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        // PageInfo type
        writeln!(
            &mut output,
            "
type PageInfo {{"
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "  hasNextPage: Boolean!")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "  hasPreviousPage: Boolean!")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "  startCursor: String")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "  endCursor: String").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "}}").map_err(Self::fmt_error_to_generator_error)?;

        // Generate connection and edge types for each non-abstract class
        for (class_name, class) in &schema.classes {
            if class.abstract_ == Some(true) {
                continue;
            }

            let type_name = self.convert_identifier(class_name);

            // Edge type
            writeln!(
                &mut output,
                "
type {type_name}Edge {{"
            )
            .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "  node: {type_name}!")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "  cursor: String!")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "}}").map_err(Self::fmt_error_to_generator_error)?;

            // Connection type
            writeln!(
                &mut output,
                "
type {type_name}Connection {{"
            )
            .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "  edges: [{type_name}Edge!]!")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "  pageInfo: PageInfo!")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "  totalCount: Int!")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "}}").map_err(Self::fmt_error_to_generator_error)?;
        }

        Ok(output)
    }

    /// Generate filter types
    fn generate_filter_types(
        &self,
        schema: &SchemaDefinition,
        indent: &IndentStyle,
    ) -> GeneratorResult<String> {
        let mut output = String::new();

        writeln!(
            &mut output,
            "
# Filter Types"
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        for (class_name, class) in &schema.classes {
            if class.abstract_ == Some(true) {
                continue;
            }

            let type_name = self.convert_identifier(class_name);
            writeln!(
                &mut output,
                "
input {type_name}Filter {{"
            )
            .map_err(Self::fmt_error_to_generator_error)?;

            // Add standard filters
            writeln!(
                &mut output,
                "{}AND: [{}Filter!]",
                indent.single(),
                type_name
            )
            .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "{}OR: [{}Filter!]", indent.single(), type_name)
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "{}NOT: {}Filter", indent.single(), type_name)
                .map_err(Self::fmt_error_to_generator_error)?;

            // Add field-specific filters
            let slots = self.collect_all_slots(class, schema)?;
            for slot_name in &slots {
                if let Some(slot) = schema.slots.get(slot_name) {
                    let field_name = Self::convert_field_name(slot_name);
                    let base_type = self.get_base_graphql_type(slot.range.as_ref());

                    match base_type.as_str() {
                        "String" => {
                            writeln!(
                                &mut output,
                                "{}{}: StringFilter",
                                indent.single(),
                                field_name
                            )
                            .map_err(Self::fmt_error_to_generator_error)?;
                        }
                        "Int" | "Float" => {
                            writeln!(
                                &mut output,
                                "{}{}: NumberFilter",
                                indent.single(),
                                field_name
                            )
                            .map_err(Self::fmt_error_to_generator_error)?;
                        }
                        "Boolean" => {
                            writeln!(
                                &mut output,
                                "{}{}: BooleanFilter",
                                indent.single(),
                                field_name
                            )
                            .map_err(Self::fmt_error_to_generator_error)?;
                        }
                        _ => {
                            writeln!(&mut output, "{}{}: IDFilter", indent.single(), field_name)
                                .map_err(Self::fmt_error_to_generator_error)?;
                        }
                    }
                }
            }

            writeln!(&mut output, "}}").map_err(Self::fmt_error_to_generator_error)?;
        }

        // Generate common filter types
        writeln!(
            &mut output,
            "
input StringFilter {{"
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "{}eq: String", indent.single())
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "{}ne: String", indent.single())
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "{}contains: String", indent.single())
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "{}startsWith: String", indent.single())
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "{}endsWith: String", indent.single())
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "{}in: [String!]", indent.single())
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "{}notIn: [String!]", indent.single())
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "}}").map_err(Self::fmt_error_to_generator_error)?;

        writeln!(
            &mut output,
            "
input NumberFilter {{"
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "{}eq: Float", indent.single())
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "{}ne: Float", indent.single())
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "{}gt: Float", indent.single())
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "{}gte: Float", indent.single())
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "{}lt: Float", indent.single())
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "{}lte: Float", indent.single())
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "{}in: [Float!]", indent.single())
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "{}notIn: [Float!]", indent.single())
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "}}").map_err(Self::fmt_error_to_generator_error)?;

        writeln!(
            &mut output,
            "
input BooleanFilter {{"
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "{}eq: Boolean", indent.single())
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "{}ne: Boolean", indent.single())
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "}}").map_err(Self::fmt_error_to_generator_error)?;

        writeln!(
            &mut output,
            "
input IDFilter {{"
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "{}eq: ID", indent.single())
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "{}ne: ID", indent.single())
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "{}in: [ID!]", indent.single())
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "{}notIn: [ID!]", indent.single())
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "}}").map_err(Self::fmt_error_to_generator_error)?;

        Ok(output)
    }

    /// Collect interfaces implemented by a class
    fn collect_interfaces(
        &self,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> Vec<String> {
        let mut interfaces = Vec::new();

        // Add parent if it's abstract
        if let Some(parent) = &class.is_a
            && let Some(parent_class) = schema.classes.get(parent)
            && parent_class.abstract_ == Some(true)
        {
            interfaces.push(self.convert_identifier(parent));
        }

        // Add mixins that are abstract
        for mixin in &class.mixins {
            if let Some(mixin_class) = schema.classes.get(mixin)
                && mixin_class.abstract_ == Some(true)
            {
                interfaces.push(self.convert_identifier(mixin));
            }
        }

        interfaces
    }

    /// Collect all slots including inherited ones
    fn collect_all_slots(
        &self,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<Vec<String>> {
        let mut all_slots = Vec::new();
        let mut seen = HashSet::new();

        // Add direct slots
        for slot in &class.slots {
            if seen.insert(slot.clone()) {
                all_slots.push(slot.clone());
            }
        }

        // Add inherited slots
        if let Some(parent) = &class.is_a
            && let Some(parent_class) = schema.classes.get(parent)
        {
            let parent_slots = self.collect_all_slots(parent_class, schema)?;
            for slot in parent_slots {
                if seen.insert(slot.clone()) {
                    all_slots.push(slot);
                }
            }
        }

        Ok(all_slots)
    }

    /// Get GraphQL type for a slot
    fn get_graphql_type(&self, slot: &SlotDefinition, _schema: &SchemaDefinition) -> String {
        let base_type = self.get_base_graphql_type(slot.range.as_ref());

        if slot.multivalued == Some(true) {
            format!("[{base_type}!]")
        } else {
            base_type
        }
    }

    /// Get base GraphQL type from `LinkML` range
    fn get_base_graphql_type(&self, range: Option<&String>) -> String {
        match range.map(String::as_str) {
            Some("string" | "str" | "date" | "datetime" | "uri" | "url") | None => {
                "String".to_string()
            }
            Some("integer" | "int") => "Int".to_string(),
            Some("float" | "double" | "decimal") => "Float".to_string(),
            Some("boolean" | "bool") => "Boolean".to_string(),
            Some(other) => self.convert_identifier(other), // Assume it's a custom type
        }
    }

    /// Convert to camelCase
    fn to_camel_case(s: &str) -> String {
        let mut result = String::new();
        let mut capitalize_next = false;

        for (i, ch) in s.chars().enumerate() {
            if i == 0 {
                result.push(ch.to_lowercase().next().unwrap_or(ch));
            } else if ch == '_' || ch == '-' {
                capitalize_next = true;
            } else if capitalize_next {
                result.push(ch.to_uppercase().next().unwrap_or(ch));
                capitalize_next = false;
            } else {
                result.push(ch);
            }
        }

        result
    }

    /// Simple pluralization
    fn pluralize(s: &str) -> String {
        if s.ends_with('s') {
            format!("{s}es")
        } else if s.ends_with('y') {
            format!("{}ies", s.strip_suffix('y').unwrap_or(s))
        } else {
            format!("{s}s")
        }
    }

    /// Convert enum values to GraphQL format
    fn convert_enum_value(value: &str) -> String {
        // GraphQL enum values must be uppercase with underscores
        value.to_uppercase().replace(['-', ' '], "_")
    }
}

impl Default for GraphQLGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl Generator for GraphQLGenerator {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &'static str {
        "Generate GraphQL schema from LinkML schemas with full CRUD support"
    }

    fn validate_schema(&self, schema: &SchemaDefinition) -> std::result::Result<(), LinkMLError> {
        // Validate schema has required fields for GraphQL generation
        if schema.name.is_empty() {
            return Err(LinkMLError::SchemaValidationError {
                message: "Schema must have a name for GraphQL generation".to_string(),
                element: Some("schema.name".to_string()),
            });
        }

        // Validate GraphQL naming requirements
        for (class_name, _class_def) in &schema.classes {
            // GraphQL names must match [_A-Za-z][_0-9A-Za-z]*
            if let Some(first) = class_name.chars().next()
                && !first.is_ascii_alphabetic()
                && first != '_'
            {
                return Err(LinkMLError::SchemaValidationError {
                    message: format!(
                        "Class name '{class_name}' is not valid for GraphQL: must start with letter or underscore"
                    ),
                    element: Some(format!("class.{class_name}")),
                });
            }

            // Check rest of characters
            for ch in class_name.chars() {
                if !ch.is_ascii_alphanumeric() && ch != '_' {
                    return Err(LinkMLError::SchemaValidationError {
                        message: format!(
                            "Class name '{class_name}' contains invalid characters for GraphQL"
                        ),
                        element: Some(format!("class.{class_name}")),
                    });
                }
            }

            // GraphQL reserved type names
            if matches!(
                class_name.as_str(),
                "Query"
                    | "Mutation"
                    | "Subscription"
                    | "Schema"
                    | "String"
                    | "Int"
                    | "Float"
                    | "Boolean"
                    | "ID"
                    | "__Schema"
                    | "__Type"
            ) {
                return Err(LinkMLError::SchemaValidationError {
                    message: format!(
                        "Class name '{class_name}' conflicts with GraphQL reserved type"
                    ),
                    element: Some(format!("class.{class_name}")),
                });
            }
        }

        // Validate field names
        for (slot_name, _slot_def) in &schema.slots {
            // GraphQL field names follow same rules as type names
            if let Some(first) = slot_name.chars().next()
                && !first.is_ascii_alphabetic()
                && first != '_'
            {
                return Err(LinkMLError::SchemaValidationError {
                    message: format!("Slot name '{slot_name}' is not valid for GraphQL fields"),
                    element: Some(format!("slot.{slot_name}")),
                });
            }

            // GraphQL introspection fields start with __
            if slot_name.starts_with("__") {
                return Err(LinkMLError::SchemaValidationError {
                    message: format!(
                        "Slot name '{slot_name}' conflicts with GraphQL introspection naming"
                    ),
                    element: Some(format!("slot.{slot_name}")),
                });
            }
        }

        Ok(())
    }

    fn generate(&self, schema: &SchemaDefinition) -> std::result::Result<String, LinkMLError> {
        // Validate schema
        self.validate_schema(schema)?;

        // Create default options
        let options = super::options::GeneratorOptions::default();

        let mut output = String::new();
        let indent = &options.indent;

        // Header
        writeln!(&mut output, "# GraphQL Schema generated from LinkML")
            .map_err(Self::fmt_error_to_generator_error)?;
        if !schema.name.is_empty() {
            writeln!(&mut output, "# Schema: {}", schema.name)
                .map_err(Self::fmt_error_to_generator_error)?;
        }
        if let Some(desc) = &schema.description {
            writeln!(&mut output, "# Description: {desc}")
                .map_err(Self::fmt_error_to_generator_error)?;
        }
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        // Generate scalar types if needed
        if options
            .get_custom("include_scalars")
            .map(std::string::String::as_str)
            == Some("true")
        {
            writeln!(&mut output, "# Custom Scalar Types")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "scalar DateTime").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "scalar Date").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "scalar URI").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
        }

        // Generate enums
        if !schema.enums.is_empty() {
            writeln!(&mut output, "# Enums").map_err(Self::fmt_error_to_generator_error)?;
            for (enum_name, enum_def) in &schema.enums {
                let enum_output = self.generate_enum(enum_name, enum_def, &options, indent)?;
                output.push_str(&enum_output);
                writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        // Generate types and interfaces
        if !schema.classes.is_empty() {
            writeln!(&mut output, "# Types and Interfaces")
                .map_err(Self::fmt_error_to_generator_error)?;

            // First generate interfaces (abstract classes)
            for (class_name, class) in &schema.classes {
                if class.abstract_ == Some(true) {
                    let class_output =
                        self.generate_class_graphql(class_name, class, schema, &options, indent)?;
                    output.push_str(&class_output);
                    writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
                }
            }

            // Then generate concrete types
            for (class_name, class) in &schema.classes {
                if class.abstract_ != Some(true) {
                    let class_output =
                        self.generate_class_graphql(class_name, class, schema, &options, indent)?;
                    output.push_str(&class_output);
                    writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
                }
            }
        }

        // Generate input types if requested
        if options
            .get_custom("generate_inputs")
            .map(std::string::String::as_str)
            != Some("false")
        {
            let input_types = self.generate_input_types(schema, &options, indent)?;
            output.push_str(&input_types);
        }

        // Generate connection types if requested
        if options
            .get_custom("generate_connections")
            .map(std::string::String::as_str)
            != Some("false")
        {
            let connection_types = self.generate_connection_types(schema, indent)?;
            output.push_str(&connection_types);
        }

        // Generate filter types if requested
        if options
            .get_custom("generate_filters")
            .map(std::string::String::as_str)
            != Some("false")
        {
            let filter_types = self.generate_filter_types(schema, indent)?;
            output.push_str(&filter_types);
        }

        // Generate root types if requested
        if options
            .get_custom("generate_root_types")
            .map(std::string::String::as_str)
            != Some("false")
        {
            let query_type = self.generate_query_type(schema, indent)?;
            output.push_str(&query_type);

            let mutation_type = self.generate_mutation_type(schema, indent)?;
            output.push_str(&mutation_type);
        }

        Ok(output)
    }

    fn get_file_extension(&self) -> &'static str {
        "graphql"
    }

    fn get_default_filename(&self) -> &'static str {
        "schema"
    }
}

impl CodeFormatter for GraphQLGenerator {
    fn name(&self) -> &'static str {
        "graphql"
    }

    fn description(&self) -> &'static str {
        "Code formatter for graphql output with proper indentation and syntax"
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec!["graphql", "gql"]
    }

    fn format_code(&self, code: &str) -> GeneratorResult<String> {
        // Basic formatting - just ensure consistent indentation
        let mut formatted = String::new();
        let indent = "    ";
        let mut indent_level: usize = 0;

        for line in code.lines() {
            let trimmed = line.trim();

            // Skip empty lines
            if trimmed.is_empty() {
                formatted.push('\n');
                continue;
            }

            // Decrease indent for closing braces
            if trimmed.starts_with('}') || trimmed.starts_with(']') || trimmed.starts_with(')') {
                indent_level = indent_level.saturating_sub(1);
            }

            // Add proper indentation
            formatted.push_str(&indent.repeat(indent_level));
            formatted.push_str(trimmed);
            formatted.push('\n');

            // Increase indent after opening braces
            if trimmed.ends_with('{') || trimmed.ends_with('[') || trimmed.ends_with('(') {
                indent_level += 1;
            }
        }

        Ok(formatted)
    }
    fn format_doc(&self, doc: &str, indent: &IndentStyle, level: usize) -> String {
        let prefix = indent.to_string(level);
        let lines: Vec<String> = doc.lines().map(|line| format!("{prefix}{line}")).collect();
        format!(
            "{}\"\"\"\
{}\
{}\"\"\"",
            prefix,
            lines.join(
                "\
"
            ),
            prefix
        )
    }

    fn format_list<T: AsRef<str>>(
        &self,
        items: &[T],
        indent: &IndentStyle,
        level: usize,
        separator: &str,
    ) -> String {
        let prefix = indent.to_string(level);
        items
            .iter()
            .map(|item| format!("{prefix}{}", item.as_ref()))
            .collect::<Vec<_>>()
            .join(separator)
    }

    fn escape_string(&self, s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace(
                '\n', "\
",
            )
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    }

    fn convert_identifier(&self, id: &str) -> String {
        // Convert to PascalCase for GraphQL types
        let mut result = String::new();
        let mut capitalize_next = true;

        for ch in id.chars() {
            if ch == '_' || ch == '-' {
                capitalize_next = true;
            } else if capitalize_next {
                result.push(ch.to_uppercase().next().unwrap_or(ch));
                capitalize_next = false;
            } else {
                result.push(ch);
            }
        }

        result
    }
}

impl GraphQLGenerator {
    /// Convert field names to camelCase
    fn convert_field_name(name: &str) -> String {
        Self::to_camel_case(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::types::{ClassDefinition, SchemaDefinition, SlotDefinition};

    #[test]
    fn test_graphql_generation() -> anyhow::Result<()> {
        let generator = GraphQLGenerator::new();

        let mut schema = SchemaDefinition {
            id: "test".to_string(),
            name: "test_schema".to_string(),
            ..Default::default()
        };

        // Add a slot
        let slot = SlotDefinition {
            name: "name".to_string(),
            range: Some("string".to_string()),
            required: Some(true),
            ..Default::default()
        };

        schema.slots.insert("name".to_string(), slot);

        // Add a class
        let class = ClassDefinition {
            name: "Person".to_string(),
            slots: vec!["name".to_string()],
            ..Default::default()
        };

        schema.classes.insert("Person".to_string(), class);

        let output = generator
            .generate(&schema)
            .expect("should generate GraphQL output: {}");

        assert!(output.contains("type Person"));
        assert!(output.contains("name: String!"));
        Ok(())
    }

    #[test]
    fn test_field_name_conversion() {
        assert_eq!(
            GraphQLGenerator::convert_field_name("first_name"),
            "firstName"
        );
        assert_eq!(
            GraphQLGenerator::convert_field_name("FirstName"),
            "firstName"
        );
        assert_eq!(
            GraphQLGenerator::convert_field_name("first-name"),
            "firstName"
        );
    }

    #[test]
    fn test_enum_value_conversion() {
        assert_eq!(
            GraphQLGenerator::convert_enum_value("in progress"),
            "IN_PROGRESS"
        );
        assert_eq!(
            GraphQLGenerator::convert_enum_value("not-started"),
            "NOT_STARTED"
        );
        assert_eq!(
            GraphQLGenerator::convert_enum_value("completed"),
            "COMPLETED"
        );
    }
}
