//! `TypeDB` integration for `LinkML` using DBMS service
//!
//! This module provides a proper integration with `TypeDB` through the DBMS service,
//! avoiding circular dependencies while maintaining the single source of truth principle.

use super::traits::{
    DataDumper, DataInstance, DataLoader, DumpOptions, DumperError, DumperResult, LoadOptions,
    LoaderError, LoaderResult,
};
use async_trait::async_trait;
use linkml_core::prelude::*;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::fmt::Write;
use tracing::{debug, info};

/// `TypeDB` integration options
#[derive(Debug, Clone)]
pub struct TypeDBIntegrationOptions {
    /// Database name in `TypeDB`
    pub database_name: String,

    /// `TypeQL` type to `LinkML` class mapping
    pub type_mapping: HashMap<String, String>,

    /// `TypeQL` attribute to `LinkML` slot mapping (per type)
    pub attribute_mapping: HashMap<String, HashMap<String, String>>,

    /// Batch size for operations
    pub batch_size: usize,

    /// Whether to infer `LinkML` types from `TypeDB` schema
    pub infer_types: bool,

    /// Include inferred facts in results
    pub include_inferred: bool,

    /// Query timeout in milliseconds
    pub query_timeout_ms: u64,
}

impl Default for TypeDBIntegrationOptions {
    fn default() -> Self {
        let config = crate::config::get_config();
        Self {
            database_name: config.typedb.default_database.clone(),
            type_mapping: HashMap::new(),
            attribute_mapping: HashMap::new(),
            batch_size: config.typedb.batch_size,
            infer_types: true,
            include_inferred: config.typedb.include_inferred,
            query_timeout_ms: config.typedb.query_timeout_ms,
        }
    }
}

/// `TypeDB` query executor trait
///
/// This trait abstracts the execution of `TypeDB` queries, allowing the loader
/// to work with either the DBMS service or a direct `TypeDB` connection.
#[async_trait]
pub trait TypeDBQueryExecutor: Send + Sync {
    /// Execute a `TypeQL` query and return results as `JSON`
    async fn execute_query(
        &self,
        query: &str,
        database: &str,
    ) -> std::result::Result<String, Box<dyn std::error::Error>>;

    /// Execute a `TypeQL` define query (schema modification)
    async fn execute_define(
        &self,
        query: &str,
        database: &str,
    ) -> std::result::Result<(), Box<dyn std::error::Error>>;

    /// Execute a `TypeQL` insert query
    async fn execute_insert(
        &self,
        query: &str,
        database: &str,
    ) -> std::result::Result<(), Box<dyn std::error::Error>>;
}

/// `TypeDB` loader using an abstract query executor
pub struct TypeDBIntegrationLoader<E: TypeDBQueryExecutor> {
    options: TypeDBIntegrationOptions,
    executor: E,
}

impl<E: TypeDBQueryExecutor> TypeDBIntegrationLoader<E> {
    /// Create a new `TypeDB` integration loader
    pub fn new(options: TypeDBIntegrationOptions, executor: E) -> Self {
        Self { options, executor }
    }

    /// Get all entity types from `TypeDB`
    async fn get_entity_types(&self) -> LoaderResult<Vec<TypeInfo>> {
        let query = "match $x sub entity; get $x;";
        let result = self
            .executor
            .execute_query(query, &self.options.database_name)
            .await
            .map_err(|e| {
                LoaderError::Io(std::io::Error::other(format!(
                    "Failed to query entity types: {e}"
                )))
            })?;

        self.parse_type_results(&result, "entity")
    }

    /// Get all relation types from `TypeDB`
    async fn get_relation_types(&self) -> LoaderResult<Vec<TypeInfo>> {
        let query = "match $x sub relation; get $x;";
        let result = self
            .executor
            .execute_query(query, &self.options.database_name)
            .await
            .map_err(|e| {
                LoaderError::Io(std::io::Error::other(format!(
                    "Failed to query relation types: {e}"
                )))
            })?;

        self.parse_type_results(&result, "relation")
    }

    /// Parse type query results
    fn parse_type_results(
        &self,
        json_result: &str,
        root_type: &str,
    ) -> LoaderResult<Vec<TypeInfo>> {
        let parsed: Value = serde_json::from_str(json_result)
            .map_err(|e| LoaderError::Parse(format!("Failed to parse JSON: {e}")))?;

        let mut types = Vec::new();

        if let Value::Array(answers) = parsed {
            for answer in answers {
                if let Value::Object(obj) = answer
                    && let Some(Value::Object(x)) = obj.get("x")
                    && let Some(Value::String(label)) = x.get("label")
                    && label != root_type
                {
                    types.push(TypeInfo {
                        name: label.clone(),
                        abstract_: x
                            .get("abstract")
                            .and_then(linkml_core::Value::as_bool)
                            .unwrap_or(false),
                    });
                }
            }
        }

        Ok(types)
    }

    /// Get attributes owned by a type
    async fn get_type_attributes(&self, type_name: &str) -> LoaderResult<Vec<AttributeInfo>> {
        let query = format!("match $type type {type_name}; $type owns $attr; get $attr;");

        let result = self
            .executor
            .execute_query(&query, &self.options.database_name)
            .await
            .map_err(|e| {
                LoaderError::Io(std::io::Error::other(format!(
                    "Failed to query attributes for {type_name}: {e}"
                )))
            })?;

        self.parse_attribute_results(&result)
    }

    /// Parse attribute query results
    fn parse_attribute_results(&self, json_result: &str) -> LoaderResult<Vec<AttributeInfo>> {
        let parsed: Value = serde_json::from_str(json_result)
            .map_err(|e| LoaderError::Parse(format!("Failed to parse JSON: {e}")))?;

        let mut attributes = Vec::new();

        if let Value::Array(answers) = parsed {
            for answer in answers {
                if let Value::Object(obj) = answer
                    && let Some(Value::Object(attr)) = obj.get("attr")
                    && let Some(Value::String(label)) = attr.get("label")
                {
                    let value_type = attr
                        .get("value_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("string")
                        .to_string();

                    attributes.push(AttributeInfo {
                        name: label.clone(),
                        _value_type: value_type,
                    });
                }
            }
        }

        Ok(attributes)
    }

    /// Get roles for a relation type
    async fn get_relation_roles(&self, relation_name: &str) -> LoaderResult<Vec<RoleInfo>> {
        let query = format!("match $rel type {relation_name}; $rel relates $role; get $role;");

        let result = self
            .executor
            .execute_query(&query, &self.options.database_name)
            .await
            .map_err(|e| {
                LoaderError::Io(std::io::Error::other(format!(
                    "Failed to query roles for {relation_name}: {e}"
                )))
            })?;

        self.parse_role_results(&result)
    }

    /// Parse role query results
    fn parse_role_results(&self, json_result: &str) -> LoaderResult<Vec<RoleInfo>> {
        let parsed: Value = serde_json::from_str(json_result)
            .map_err(|e| LoaderError::Parse(format!("Failed to parse JSON: {e}")))?;

        let mut roles = Vec::new();

        if let Value::Array(answers) = parsed {
            for answer in answers {
                if let Value::Object(obj) = answer
                    && let Some(Value::Object(role)) = obj.get("role")
                    && let Some(Value::String(label)) = role.get("label")
                {
                    roles.push(RoleInfo {
                        _name: label.clone(),
                    });
                }
            }
        }

        Ok(roles)
    }

    /// Load instances of a specific type
    async fn load_type_instances(
        &self,
        type_info: &TypeInfo,
        attributes: &[AttributeInfo],
    ) -> LoaderResult<Vec<DataInstance>> {
        // Skip abstract types
        if type_info.abstract_ {
            return Ok(Vec::new());
        }

        let class_name = self
            .options
            .type_mapping
            .get(&type_info.name)
            .cloned()
            .unwrap_or_else(|| to_pascal_case(&type_info.name));

        // Build the match query
        let mut query = format!("match $x isa {};", type_info.name);
        for attr in attributes {
            write!(query, " $x has {} $attr_{};", attr.name, attr.name)
                .expect("write! to String should never fail");
        }
        query.push_str(" get $x");
        for attr in attributes {
            write!(query, ", $attr_{}", attr.name).expect("write! to String should never fail");
        }
        query.push(';');

        if self.options.include_inferred {
            query = format!("match {} infer true;", query.trim_start_matches("match "));
        }

        debug!("Executing query: {}", query);

        let result = self
            .executor
            .execute_query(&query, &self.options.database_name)
            .await
            .map_err(|e| {
                LoaderError::Io(std::io::Error::other(format!(
                    "Failed to query instances of {}: {}",
                    type_info.name, e
                )))
            })?;

        self.parse_instance_results(&result, &class_name, &type_info.name, attributes)
    }

    /// Parse instance query results
    fn parse_instance_results(
        &self,
        json_result: &str,
        class_name: &str,
        type_name: &str,
        attributes: &[AttributeInfo],
    ) -> LoaderResult<Vec<DataInstance>> {
        let parsed: Value = serde_json::from_str(json_result)
            .map_err(|e| LoaderError::Parse(format!("Failed to parse JSON: {e}")))?;

        let mut instances = Vec::new();

        if let Value::Array(answers) = parsed {
            for answer in answers {
                if let Value::Object(obj) = answer {
                    let mut data = Map::new();

                    // Extract the instance ID
                    if let Some(Value::Object(x)) = obj.get("x")
                        && let Some(Value::String(iid)) = x.get("iid")
                    {
                        data.insert("_typedb_iid".to_string(), Value::String(iid.clone()));
                    }

                    // Extract attribute values
                    for attr in attributes {
                        let var_name = format!("attr_{}", attr.name);
                        if let Some(Value::Object(attr_obj)) = obj.get(&var_name)
                            && let Some(value) = attr_obj.get("value")
                        {
                            let slot_name = self.get_slot_name(type_name, &attr.name);
                            data.insert(slot_name, value.clone());
                        }
                    }

                    instances.push(DataInstance {
                        class_name: class_name.to_string(),
                        data: data.into_iter().collect(),
                        id: None,
                        metadata: HashMap::new(),
                    });
                }
            }
        }

        Ok(instances)
    }

    /// Get the `LinkML` slot name for a `TypeDB` attribute
    fn get_slot_name(&self, type_name: &str, attr_name: &str) -> String {
        if let Some(type_mapping) = self.options.attribute_mapping.get(type_name)
            && let Some(slot_name) = type_mapping.get(attr_name)
        {
            return slot_name.clone();
        }
        to_snake_case(attr_name)
    }
}

#[async_trait]
impl<E: TypeDBQueryExecutor> DataLoader for TypeDBIntegrationLoader<E> {
    fn name(&self) -> &'static str {
        "typedb-integration"
    }

    fn description(&self) -> &'static str {
        "Load data from TypeDB with full integration support"
    }

    fn supported_extensions(&self) -> Vec<&str> {
        vec![] // TypeDB loader doesn't use file extensions
    }

    async fn load_file(
        &self,
        _path: &std::path::Path,
        _schema: &SchemaDefinition,
        _options: &LoadOptions,
    ) -> LoaderResult<Vec<DataInstance>> {
        Err(LoaderError::InvalidFormat(
            "TypeDB loader does not support file loading".to_string(),
        ))
    }

    async fn load_string(
        &self,
        _content: &str,
        _schema: &SchemaDefinition,
        _options: &LoadOptions,
    ) -> LoaderResult<Vec<DataInstance>> {
        // Get all types
        let entity_types = self.get_entity_types().await?;
        info!("Found {} entity types", entity_types.len());

        let relation_types = self.get_relation_types().await?;
        info!("Found {} relation types", relation_types.len());

        let mut all_instances = Vec::new();

        // Load entities
        for type_info in &entity_types {
            if type_info.abstract_ {
                debug!("Skipping abstract entity type: {}", type_info.name);
                continue;
            }

            debug!("Loading instances of entity type: {}", type_info.name);
            let attributes = self.get_type_attributes(&type_info.name).await?;
            let instances = self.load_type_instances(type_info, &attributes).await?;
            info!(
                "Loaded {} instances of type {}",
                instances.len(),
                type_info.name
            );
            all_instances.extend(instances);
        }

        // Load relations
        for type_info in &relation_types {
            if type_info.abstract_ {
                debug!("Skipping abstract relation type: {}", type_info.name);
                continue;
            }

            debug!("Loading instances of relation type: {}", type_info.name);
            let attributes = self.get_type_attributes(&type_info.name).await?;
            let _roles = self.get_relation_roles(&type_info.name).await?;

            // For now, just load the relation instances without role players
            // Full role player loading would require more complex queries
            let instances = self.load_type_instances(type_info, &attributes).await?;
            info!(
                "Loaded {} instances of relation {}",
                instances.len(),
                type_info.name
            );
            all_instances.extend(instances);
        }

        Ok(all_instances)
    }

    async fn load_bytes(
        &self,
        _data: &[u8],
        _schema: &SchemaDefinition,
        _options: &LoadOptions,
    ) -> LoaderResult<Vec<DataInstance>> {
        Err(LoaderError::InvalidFormat(
            "TypeDB loader does not support raw bytes loading".to_string(),
        ))
    }

    fn validate_schema(&self, _schema: &SchemaDefinition) -> LoaderResult<()> {
        Ok(())
    }
}

/// `TypeDB` dumper using an abstract query executor
pub struct TypeDBIntegrationDumper<E: TypeDBQueryExecutor> {
    options: TypeDBIntegrationOptions,
    executor: E,
}

impl<E: TypeDBQueryExecutor> TypeDBIntegrationDumper<E> {
    /// Create a new `TypeDB` integration dumper
    pub fn new(options: TypeDBIntegrationOptions, executor: E) -> Self {
        Self { options, executor }
    }

    /// Create `TypeDB` schema for a `LinkML` class
    async fn create_schema_if_needed(
        &self,
        class_name: &str,
        class_def: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> DumperResult<()> {
        let type_name = self
            .options
            .type_mapping
            .iter()
            .find(|(_, cn)| cn == &class_name)
            .map_or_else(|| to_snake_case(class_name), |(tn, _)| tn.clone());

        // Determine if this is a relation or entity
        let is_relation = self.is_relation_class(class_def, schema);

        let mut define_query = String::new();

        if is_relation {
            write!(define_query, "define {type_name} sub relation")
                .expect("write! to String should never fail");

            // Add roles based on object-valued slots
            for slot_name in &class_def.slots {
                if let Some(slot_def) = schema.slots.get(slot_name)
                    && let Some(range) = &slot_def.range
                    && schema.classes.contains_key(range)
                {
                    let role_name = to_snake_case(slot_name);
                    write!(define_query, ", relates {role_name}")
                        .expect("write! to String should never fail");
                }
            }
        } else {
            write!(define_query, "define {type_name} sub entity")
                .expect("write! to String should never fail");
        }

        // Add attributes
        for slot_name in &class_def.slots {
            if let Some(slot_def) = schema.slots.get(slot_name)
                && let Some(range) = &slot_def.range
                && !schema.classes.contains_key(range)
            {
                let attr_name = to_snake_case(slot_name);
                let value_type = linkml_range_to_typedb_value_type(range);

                // Define attribute type if needed
                write!(
                    define_query,
                    "; {attr_name} sub attribute, value {value_type}"
                )
                .expect("LinkML operation should succeed");

                // Type owns attribute
                write!(define_query, "; {type_name} owns {attr_name}")
                    .expect("write! to String should never fail");
            }
        }

        define_query.push(';');

        self.executor
            .execute_define(&define_query, &self.options.database_name)
            .await
            .map_err(|e| {
                DumperError::Io(std::io::Error::other(format!(
                    "Failed to define schema: {e}"
                )))
            })?;

        Ok(())
    }

    /// Check if a class represents a relation
    fn is_relation_class(&self, class_def: &ClassDefinition, schema: &SchemaDefinition) -> bool {
        class_def.slots.iter().any(|slot_name| {
            schema
                .slots
                .get(slot_name)
                .and_then(|slot| slot.range.as_ref())
                .is_some_and(|range| schema.classes.contains_key(range))
        })
    }

    /// Insert instances into `TypeDB`
    async fn insert_instances(
        &self,
        class_name: &str,
        instances: &[DataInstance],
        schema: &SchemaDefinition,
    ) -> DumperResult<()> {
        let type_name = self
            .options
            .type_mapping
            .iter()
            .find(|(_, cn)| cn == &class_name)
            .map_or_else(|| to_snake_case(class_name), |(tn, _)| tn.clone());

        let class_def = schema.classes.get(class_name).ok_or_else(|| {
            DumperError::SchemaValidation(format!("Class {class_name} not found in schema"))
        })?;

        let is_relation = self.is_relation_class(class_def, schema);

        // Process in batches
        for batch in instances.chunks(self.options.batch_size) {
            let mut queries = Vec::new();

            for instance in batch {
                let query = if is_relation {
                    self.build_relation_insert_query(&type_name, instance, schema)?
                } else {
                    self.build_entity_insert_query(&type_name, instance, schema)?
                };

                queries.push(query);
            }

            // Execute all queries in the batch
            for query in queries {
                self.executor
                    .execute_insert(&query, &self.options.database_name)
                    .await
                    .map_err(|e| {
                        DumperError::Io(std::io::Error::other(format!(
                            "Failed to insert instance: {e}"
                        )))
                    })?;
            }
        }

        Ok(())
    }

    /// Build insert query for an entity
    fn build_entity_insert_query(
        &self,
        type_name: &str,
        instance: &DataInstance,
        schema: &SchemaDefinition,
    ) -> DumperResult<String> {
        let mut query = format!("insert $x isa {type_name}");

        for (slot_name, value) in &instance.data {
            if slot_name.starts_with('_') {
                continue;
            }

            // Skip object-valued slots
            if let Some(slot_def) = schema.slots.get(slot_name)
                && let Some(range) = &slot_def.range
                && schema.classes.contains_key(range)
            {
                continue;
            }

            let attr_name = to_snake_case(slot_name);
            let typeql_value = json_value_to_typeql(value)?;
            write!(query, ", has {attr_name} {typeql_value}")
                .expect("write! to String should never fail");
        }

        query.push(';');
        Ok(query)
    }

    /// Build insert query for a relation
    fn build_relation_insert_query(
        &self,
        type_name: &str,
        instance: &DataInstance,
        schema: &SchemaDefinition,
    ) -> DumperResult<String> {
        let mut match_part = String::from("match ");
        let mut role_players = Vec::new();

        // Match role players
        for (slot_name, value) in &instance.data {
            if let Some(slot_def) = schema.slots.get(slot_name)
                && let Some(range) = &slot_def.range
                && schema.classes.contains_key(range)
            {
                // This is a role player
                if let Value::Object(obj) = value
                    && let Some(Value::String(id)) = obj.get("id")
                {
                    let role_type = to_snake_case(range);
                    write!(
                        match_part,
                        "${slot_name} isa {role_type}, has id \"{id}\"; "
                    )
                    .expect("LinkML operation should succeed");
                    role_players.push((to_snake_case(slot_name), slot_name.clone()));
                }
            }
        }

        // Build insert part
        let mut insert_part = format!(
            "insert $rel ({}) isa {}",
            role_players
                .iter()
                .map(|(role, var)| format!("{role}: ${var}"))
                .collect::<Vec<_>>()
                .join(", "),
            type_name
        );

        // Add attributes
        for (slot_name, value) in &instance.data {
            if slot_name.starts_with('_') {
                continue;
            }

            // Skip object-valued slots
            if let Some(slot_def) = schema.slots.get(slot_name)
                && let Some(range) = &slot_def.range
                && schema.classes.contains_key(range)
            {
                continue;
            }

            let attr_name = to_snake_case(slot_name);
            let typeql_value = json_value_to_typeql(value)?;
            write!(insert_part, ", has {attr_name} {typeql_value}")
                .expect("write! to String should never fail");
        }

        Ok(format!("{match_part} {insert_part}"))
    }
}

#[async_trait]
impl<E: TypeDBQueryExecutor> DataDumper for TypeDBIntegrationDumper<E> {
    fn name(&self) -> &'static str {
        "typedb-integration"
    }

    fn description(&self) -> &'static str {
        "Dump data to TypeDB with full integration support"
    }

    fn supported_extensions(&self) -> Vec<&str> {
        vec![] // TypeDB dumper doesn't use file extensions
    }

    async fn dump_file(
        &self,
        _instances: &[DataInstance],
        _path: &std::path::Path,
        _schema: &SchemaDefinition,
        _options: &DumpOptions,
    ) -> DumperResult<()> {
        Err(DumperError::Configuration(
            "TypeDB dumper does not support file dumping".to_string(),
        ))
    }

    async fn dump_string(
        &self,
        instances: &[DataInstance],
        schema: &SchemaDefinition,
        _options: &DumpOptions,
    ) -> DumperResult<String> {
        // Group instances by class
        let mut instances_by_class: HashMap<String, Vec<&DataInstance>> = HashMap::new();
        for instance in instances {
            instances_by_class
                .entry(instance.class_name.clone())
                .or_default()
                .push(instance);
        }

        // Create schemas and insert data
        for (class_name, class_instances) in instances_by_class {
            if let Some(class_def) = schema.classes.get(&class_name) {
                // Create schema if needed
                self.create_schema_if_needed(&class_name, class_def, schema)
                    .await?;

                // Convert references to owned instances
                let owned_instances: Vec<DataInstance> =
                    class_instances.into_iter().cloned().collect();

                // Insert instances
                self.insert_instances(&class_name, &owned_instances, schema)
                    .await?;

                info!(
                    "Dumped {} instances of class {}",
                    owned_instances.len(),
                    class_name
                );
            }
        }

        let summary = format!(
            "Successfully dumped {} instances to TypeDB",
            instances.len()
        );
        Ok(summary)
    }

    async fn dump_bytes(
        &self,
        instances: &[DataInstance],
        schema: &SchemaDefinition,
        options: &DumpOptions,
    ) -> DumperResult<Vec<u8>> {
        let result = self.dump_string(instances, schema, options).await?;
        Ok(result.into_bytes())
    }

    fn validate_schema(&self, _schema: &SchemaDefinition) -> DumperResult<()> {
        Ok(())
    }
}

// Helper structures
#[derive(Debug, Clone)]
struct TypeInfo {
    name: String,
    abstract_: bool,
}

#[derive(Debug, Clone)]
struct AttributeInfo {
    name: String,
    _value_type: String,
}

#[derive(Debug, Clone)]
struct RoleInfo {
    _name: String,
}

// Helper functions
fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut prev_upper = false;

    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 && !prev_upper {
            result.push('_');
        }
        // to_lowercase() returns an iterator that always has at least one char
        result.push(
            ch.to_lowercase()
                .next()
                .expect("lowercase char should exist"),
        );
        prev_upper = ch.is_uppercase();
    }

    result
}

fn linkml_range_to_typedb_value_type(range: &str) -> &str {
    match range {
        "integer" => "long",
        "float" => "double",
        "boolean" => "boolean",
        "string" => "string",
        "date" | "datetime" => "datetime",
        "time" => "datetime",
        _ => "string",
    }
}

fn json_value_to_typeql(value: &Value) -> DumperResult<String> {
    match value {
        Value::String(s) => Ok(format!("\"{}\"", s.replace('\"', "\\\""))),
        Value::Number(n) => Ok(n.to_string()),
        Value::Bool(b) => Ok(b.to_string()),
        Value::Null => Err(DumperError::TypeConversion(
            "Cannot insert null values into TypeDB".to_string(),
        )),
        _ => Err(DumperError::TypeConversion(format!(
            "Unsupported value type: {value:?}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integration_options_default() {
        let options = TypeDBIntegrationOptions::default();

        assert_eq!(options.batch_size, 1000);
        assert!(options.infer_types);
        assert!(!options.include_inferred);
        // Default timeout is 10000ms from config
        assert_eq!(options.query_timeout_ms, 10000);
    }

    #[test]
    fn test_case_conversions() {
        assert_eq!(to_pascal_case("user_account"), "UserAccount");
        assert_eq!(to_pascal_case("employment"), "Employment");

        assert_eq!(to_snake_case("UserAccount"), "user_account");
        assert_eq!(to_snake_case("Employment"), "employment");
    }

    #[test]
    fn test_type_conversions() {
        assert_eq!(linkml_range_to_typedb_value_type("integer"), "long");
        assert_eq!(linkml_range_to_typedb_value_type("float"), "double");
        assert_eq!(linkml_range_to_typedb_value_type("boolean"), "boolean");
        assert_eq!(linkml_range_to_typedb_value_type("string"), "string");
        assert_eq!(linkml_range_to_typedb_value_type("datetime"), "datetime");
    }

    #[test]
    fn test_json_to_typeql() -> std::result::Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            json_value_to_typeql(&Value::String("test".to_string()))?,
            "\"test\""
        );
        assert_eq!(json_value_to_typeql(&Value::Number(42.into()))?, "42");
        assert_eq!(json_value_to_typeql(&Value::Bool(true))?, "true");
        assert!(json_value_to_typeql(&Value::Null).is_err());
        Ok(())
    }
}
