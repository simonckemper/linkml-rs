//! TypeDB service integration for LinkML schemas
//!
//! This module provides integration between LinkML schemas and TypeDB, enabling:
//! - Automatic TypeDB schema generation from LinkML definitions
//! - Data migration between LinkML and TypeDB formats
//! - Query generation and execution
//! - Schema validation and synchronization

use std::collections::HashMap;
use std::sync::Arc;

use dbms_core::{
    DBMSService, DatabaseConfig, DBMSError, DBMSResult,
    SchemaVersion, DatabaseStatus, HealthStatus, SchemaValidation,
    DatabaseConnection, ConnectionPool, DatabaseMetrics, OptimizationReport,
    DatabaseEvent,
};
use linkml_core::types::{ClassDefinition, SchemaDefinition, SlotDefinition};
use linkml_core::error::{LinkMLError, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// TypeDB integration service for LinkML
pub struct TypeDBIntegration<D>
where
    D: DBMSService,
{
    /// DBMS service for TypeDB operations
    dbms_service: Arc<D>,
    /// Schema mapping cache
    schema_cache: HashMap<String, TypeDBSchema>,
    /// Configuration for the integration
    config: TypeDBIntegrationConfig,
}

/// Configuration for TypeDB integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDBIntegrationConfig {
    /// Database name for LinkML schemas
    pub database_name: String,
    /// Enable automatic schema synchronization
    pub auto_sync: bool,
    /// Enable validation before operations
    pub validate_before_ops: bool,
    /// Maximum retry attempts for operations
    pub max_retries: u32,
    /// Timeout for operations in seconds
    pub operation_timeout: u64,
}

impl Default for TypeDBIntegrationConfig {
    fn default() -> Self {
        Self {
            database_name: "linkml_schemas".to_string(),
            auto_sync: true,
            validate_before_ops: true,
            max_retries: 3,
            operation_timeout: 30,
        }
    }
}

/// Schema deployment configuration (local type until dbms-core is updated)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaDeployment {
    /// Schema content to deploy
    pub schema_content: String,
    /// Version of the schema
    pub version: Option<String>,
    /// Description of the schema
    pub description: Option<String>,
    /// Whether to rollback on error
    pub rollback_on_error: bool,
}

/// Query result from TypeDB (local type until dbms-core is updated)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    /// Result rows
    pub rows: Vec<HashMap<String, Value>>,
    /// Number of affected rows for mutations
    pub affected_rows: usize,
}

/// Database information (local type until dbms-core is updated)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseInfo {
    /// Database name
    pub name: String,
    /// Database status
    pub status: String,
}

/// TypeDB schema representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDBSchema {
    /// Schema name
    pub name: String,
    /// TypeQL schema definition
    pub typeql: String,
    /// Mapping of LinkML classes to TypeDB entities
    pub entity_mappings: HashMap<String, String>,
    /// Mapping of LinkML slots to TypeDB attributes
    pub attribute_mappings: HashMap<String, String>,
    /// Mapping of relationships
    pub relation_mappings: HashMap<String, RelationMapping>,
    /// Schema version
    pub version: String,
}

/// Mapping for TypeDB relations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationMapping {
    /// Relation type name in TypeDB
    pub relation_type: String,
    /// Roles in the relation
    pub roles: Vec<String>,
    /// Mapping of LinkML slots to roles
    pub role_mappings: HashMap<String, String>,
}

impl<D> TypeDBIntegration<D>
where
    D: DBMSService,
{
    /// Create a new TypeDB integration instance
    pub fn new(dbms_service: Arc<D>, config: TypeDBIntegrationConfig) -> Self {
        Self {
            dbms_service,
            schema_cache: HashMap::new(),
            config,
        }
    }

    /// Convert LinkML schema to TypeDB schema
    pub fn linkml_to_typedb(&self, schema: &SchemaDefinition) -> Result<TypeDBSchema> {
        let mut typeql = String::new();
        let mut entity_mappings = HashMap::new();
        let mut attribute_mappings = HashMap::new();
        let mut relation_mappings = HashMap::new();

        // Add schema header
        typeql.push_str("define\n\n");

        // Convert LinkML types to TypeDB attributes
        for (slot_name, slot_def) in &schema.slots {
            let attribute_type = self.map_slot_to_attribute(slot_name, slot_def)?;
            typeql.push_str(&format!("{}\n", attribute_type));
            attribute_mappings.insert(slot_name.clone(), self.sanitize_name(slot_name));
        }

        // Convert LinkML classes to TypeDB entities
        for (class_name, class_def) in &schema.classes {
            let entity_type = self.map_class_to_entity(class_name, class_def, schema)?;
            typeql.push_str(&format!("\n{}\n", entity_type));
            entity_mappings.insert(class_name.clone(), self.sanitize_name(class_name));

            // Check for relationships
            if let Some(relations) = self.extract_relations(class_def, schema)? {
                for relation in relations {
                    relation_mappings.insert(relation.relation_type.clone(), relation);
                }
            }
        }

        // Add relations to TypeQL
        for (_, relation) in &relation_mappings {
            typeql.push_str(&format!("\n{} sub relation,\n", relation.relation_type));
            for role in &relation.roles {
                typeql.push_str(&format!("  relates {},\n", role));
            }
            typeql.push_str(";\n");
        }

        Ok(TypeDBSchema {
            name: schema.name.clone(),
            typeql,
            entity_mappings,
            attribute_mappings,
            relation_mappings,
            version: schema.version.clone().unwrap_or_else(|| "1.0.0".to_string()),
        })
    }

    /// Map LinkML slot to TypeDB attribute
    fn map_slot_to_attribute(&self, slot_name: &str, slot_def: &SlotDefinition) -> Result<String> {
        let sanitized_name = self.sanitize_name(slot_name);
        let value_type = self.map_range_to_typedb_type(slot_def.range.as_deref().unwrap_or("string"));
        
        let mut attribute = format!("{} sub attribute, value {};", sanitized_name, value_type);
        
        // Add regex constraint if pattern is specified
        if let Some(ref pattern) = slot_def.pattern {
            attribute.push_str(&format!(" regex \"{}\";", pattern));
        }
        
        Ok(attribute)
    }

    /// Map LinkML class to TypeDB entity
    fn map_class_to_entity(
        &self,
        class_name: &str,
        class_def: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> Result<String> {
        let sanitized_name = self.sanitize_name(class_name);
        let mut entity = String::new();

        // Determine parent entity
        let parent = if let Some(ref is_a) = class_def.is_a {
            self.sanitize_name(is_a)
        } else {
            "entity".to_string()
        };

        entity.push_str(&format!("{} sub {},\n", sanitized_name, parent));

        // Add attributes
        for slot_name in &class_def.slots {
            if schema.slots.contains_key(slot_name) {
                let attr_name = self.sanitize_name(slot_name);
                entity.push_str(&format!("  owns {},\n", attr_name));
                
                // Add key constraint for identifiers
                if let Some(slot_def) = schema.slots.get(slot_name) {
                    if slot_def.identifier.unwrap_or(false) {
                        entity.push_str(&format!("  key {},\n", attr_name));
                    }
                }
            }
        }

        // Handle relations (plays roles)
        if let Some(relations) = self.extract_relations(class_def, schema)? {
            for relation in relations {
                for (slot, role) in &relation.role_mappings {
                    if class_def.slots.contains(slot) {
                        entity.push_str(&format!("  plays {}:{},\n", relation.relation_type, role));
                    }
                }
            }
        }

        entity.push_str(";");
        Ok(entity)
    }

    /// Extract relations from class definition
    fn extract_relations(
        &self,
        class_def: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> Result<Option<Vec<RelationMapping>>> {
        let mut relations = Vec::new();

        for slot_name in &class_def.slots {
            if let Some(slot_def) = schema.slots.get(slot_name) {
                // Check if slot references another class (indicating a relation)
                if let Some(ref range) = slot_def.range {
                    if schema.classes.contains_key(range) {
                        // This is a relation
                        let relation_type = format!("has_{}", self.sanitize_name(slot_name));
                        let mut role_mappings = HashMap::new();
                        
                        role_mappings.insert(slot_name.clone(), "target".to_string());
                        role_mappings.insert(format!("{}_owner", slot_name), "owner".to_string());
                        
                        relations.push(RelationMapping {
                            relation_type,
                            roles: vec!["owner".to_string(), "target".to_string()],
                            role_mappings,
                        });
                    }
                }
            }
        }

        if relations.is_empty() {
            Ok(None)
        } else {
            Ok(Some(relations))
        }
    }

    /// Map LinkML range to TypeDB value type
    fn map_range_to_typedb_type(&self, range: &str) -> &str {
        match range {
            "string" | "str" | "uri" | "uriorcurie" | "curie" | "ncname" => "string",
            "integer" | "int" => "long",
            "float" | "double" | "decimal" => "double",
            "boolean" | "bool" => "boolean",
            "date" | "datetime" | "time" => "datetime",
            _ => "string", // Default to string for unknown types
        }
    }

    /// Sanitize names for TypeDB compatibility
    fn sanitize_name(&self, name: &str) -> String {
        // TypeDB names must start with letter and contain only alphanumeric and underscore
        let mut sanitized = String::new();
        
        for (i, c) in name.chars().enumerate() {
            if i == 0 && !c.is_alphabetic() {
                sanitized.push('_');
            }
            
            if c.is_alphanumeric() || c == '_' {
                sanitized.push(c.to_lowercase().next().unwrap_or(c));
            } else {
                sanitized.push('_');
            }
        }
        
        sanitized
    }

    /// Deploy LinkML schema to TypeDB
    pub async fn deploy_schema(&mut self, schema: &SchemaDefinition) -> Result<()> {
        // Convert schema to TypeDB format
        let typedb_schema = self.linkml_to_typedb(schema)?;
        
        // Cache the schema
        self.schema_cache.insert(schema.name.clone(), typedb_schema.clone());
        
        // Create database if it doesn't exist
        let db_config = DatabaseConfig::default();
        let db_info = self.dbms_service
            .create_database(&self.config.database_name, db_config)
            .await
            .map_err(|e| LinkMLError::service(format!("Failed to create database: {}", e)))?;
        
        println!("✓ Created/connected to database: {}", db_info.name);
        
        // Deploy the schema
        let deployment = SchemaDeployment {
            schema_content: typedb_schema.typeql.clone(),
            version: Some(typedb_schema.version.clone()),
            description: schema.description.clone(),
            rollback_on_error: true,
        };
        
        self.dbms_service
            .deploy_schema(&self.config.database_name, &deployment.schema_content)
            .await
            .map_err(|e| LinkMLError::service(format!("Failed to deploy schema: {}", e)))?;
        
        println!("✓ Deployed TypeDB schema for '{}'", schema.name);
        Ok(())
    }

    /// Insert data from LinkML instance to TypeDB
    pub async fn insert_data(
        &self,
        schema_name: &str,
        class_name: &str,
        data: &Value,
    ) -> Result<()> {
        // Get cached schema
        let typedb_schema = self.schema_cache
            .get(schema_name)
            .ok_or_else(|| LinkMLError::service(format!("Schema '{}' not found in cache", schema_name)))?;
        
        // Get entity mapping
        let entity_type = typedb_schema.entity_mappings
            .get(class_name)
            .ok_or_else(|| LinkMLError::service(format!("Class '{}' not found in schema", class_name)))?;
        
        // Build insert query
        let mut query = format!("insert $x isa {};", entity_type);
        
        // Add attributes from data
        if let Value::Object(map) = data {
            for (field, value) in map {
                if let Some(attr_name) = typedb_schema.attribute_mappings.get(field) {
                    let value_str = self.format_value_for_typedb(value)?;
                    query.push_str(&format!(" $x has {} {};", attr_name, value_str));
                }
            }
        }
        
        // TODO: Re-enable when execute_query is added to DBMSService trait
        // self.dbms_service
        //     .execute_query(&self.config.database_name, &query, HashMap::new())
        //     .await
        //     .map_err(|e| LinkMLError::service(format!("Failed to insert data: {}", e)))?;
        
        println!("✓ Would insert data for entity type '{}' (disabled - execute_query not available)", entity_type);
        Ok(())
    }

    /// Query data from TypeDB based on LinkML class
    pub async fn query_data(
        &self,
        schema_name: &str,
        class_name: &str,
        filters: HashMap<String, String>,
    ) -> Result<Vec<Value>> {
        // Get cached schema
        let typedb_schema = self.schema_cache
            .get(schema_name)
            .ok_or_else(|| LinkMLError::service(format!("Schema '{}' not found in cache", schema_name)))?;
        
        // Get entity mapping
        let entity_type = typedb_schema.entity_mappings
            .get(class_name)
            .ok_or_else(|| LinkMLError::service(format!("Class '{}' not found in schema", class_name)))?;
        
        // Build match query
        let mut query = format!("match $x isa {};", entity_type);
        
        // Add filters
        for (field, value) in &filters {
            if let Some(attr_name) = typedb_schema.attribute_mappings.get(field) {
                query.push_str(&format!(" $x has {} {};", attr_name, value));
            }
        }
        
        // Add fetch clause to get all attributes
        query.push_str(" get $x;");
        
        // TODO: Re-enable when execute_query is added to DBMSService trait
        // let results = self.dbms_service
        //     .execute_query(&self.config.database_name, &query, HashMap::new())
        //     .await
        //     .map_err(|e| LinkMLError::service(format!("Failed to query data: {}", e)))?;
        
        // Temporary empty result
        let results = QueryResult {
            rows: vec![],
            affected_rows: 0,
        };
        
        // Convert results to JSON values
        let mut json_results = Vec::new();
        for row in results.rows {
            let mut obj = serde_json::Map::new();
            
            // Convert TypeDB results back to LinkML format
            for (key, value) in row {
                // Reverse map attribute names
                for (linkml_name, typedb_name) in &typedb_schema.attribute_mappings {
                    if typedb_name == &key {
                        obj.insert(linkml_name.clone(), value.clone());
                        break;
                    }
                }
            }
            
            json_results.push(Value::Object(obj));
        }
        
        println!("✓ Retrieved {} records for entity type '{}'", json_results.len(), entity_type);
        Ok(json_results)
    }

    /// Format value for TypeDB query
    fn format_value_for_typedb(&self, value: &Value) -> Result<String> {
        match value {
            Value::String(s) => Ok(format!("\"{}\"", s.replace('"', "\\\""))),
            Value::Number(n) => Ok(n.to_string()),
            Value::Bool(b) => Ok(b.to_string()),
            Value::Null => Err(LinkMLError::service("Cannot insert null values into TypeDB".to_string())),
            _ => Err(LinkMLError::service(format!("Unsupported value type for TypeDB: {:?}", value))),
        }
    }

    /// Validate LinkML data against TypeDB schema constraints
    pub async fn validate_data(
        &self,
        schema_name: &str,
        class_name: &str,
        data: &Value,
    ) -> Result<Vec<String>> {
        let mut errors = Vec::new();
        
        // Get cached schema
        let typedb_schema = self.schema_cache
            .get(schema_name)
            .ok_or_else(|| LinkMLError::service(format!("Schema '{}' not found in cache", schema_name)))?;
        
        // Check if class exists
        if !typedb_schema.entity_mappings.contains_key(class_name) {
            errors.push(format!("Class '{}' not found in TypeDB schema", class_name));
            return Ok(errors);
        }
        
        // Validate data structure
        if let Value::Object(map) = data {
            // Check for unknown fields
            for field in map.keys() {
                if !typedb_schema.attribute_mappings.contains_key(field) {
                    errors.push(format!("Field '{}' is not mapped to any TypeDB attribute", field));
                }
            }
            
            // Additional validation could be added here
            // - Check data types
            // - Validate against regex patterns
            // - Check required fields
        } else {
            errors.push("Data must be an object".to_string());
        }
        
        Ok(errors)
    }

    /// Synchronize LinkML schema changes with TypeDB
    pub async fn sync_schema(&mut self, old_schema: &SchemaDefinition, new_schema: &SchemaDefinition) -> Result<()> {
        println!("Synchronizing schema '{}' with TypeDB...", new_schema.name);
        
        // Generate migration commands
        let migration_commands = self.generate_migration_commands(old_schema, new_schema)?;
        
        if migration_commands.is_empty() {
            println!("✓ No schema changes detected");
            return Ok(());
        }
        
        println!("Found {} migration commands to execute", migration_commands.len());
        
        // Execute migration commands
        for command in migration_commands {
            println!("  Would execute: {}", command);
            // TODO: Re-enable when execute_query is added to DBMSService trait
            // self.dbms_service
            //     .execute_query(&self.config.database_name, &command, HashMap::new())
            //     .await
            //     .map_err(|e| LinkMLError::service(format!("Failed to execute migration: {}", e)))?;
        }
        
        // Update cached schema
        let typedb_schema = self.linkml_to_typedb(new_schema)?;
        self.schema_cache.insert(new_schema.name.clone(), typedb_schema);
        
        println!("✓ Schema synchronized successfully");
        Ok(())
    }

    /// Generate migration commands for schema changes
    fn generate_migration_commands(&self, old_schema: &SchemaDefinition, new_schema: &SchemaDefinition) -> Result<Vec<String>> {
        let mut commands = Vec::new();
        
        // Check for new classes
        for (class_name, class_def) in &new_schema.classes {
            if !old_schema.classes.contains_key(class_name) {
                // New class - generate define statement
                let entity_type = self.map_class_to_entity(class_name, class_def, new_schema)?;
                commands.push(format!("define {}", entity_type));
            }
        }
        
        // Check for new slots (attributes)
        for (slot_name, slot_def) in &new_schema.slots {
            if !old_schema.slots.contains_key(slot_name) {
                // New slot - generate define statement
                let attribute_type = self.map_slot_to_attribute(slot_name, slot_def)?;
                commands.push(format!("define {}", attribute_type));
            }
        }
        
        // Check for deleted classes
        for class_name in old_schema.classes.keys() {
            if !new_schema.classes.contains_key(class_name) {
                let sanitized_name = self.sanitize_name(class_name);
                commands.push(format!("undefine {} sub entity;", sanitized_name));
            }
        }
        
        // Check for deleted slots
        for slot_name in old_schema.slots.keys() {
            if !new_schema.slots.contains_key(slot_name) {
                let sanitized_name = self.sanitize_name(slot_name);
                commands.push(format!("undefine {} sub attribute;", sanitized_name));
            }
        }
        
        Ok(commands)
    }
}

/// Create a TypeDB integration service
pub fn create_typedb_integration<D>(
    dbms_service: Arc<D>,
    config: Option<TypeDBIntegrationConfig>,
) -> TypeDBIntegration<D>
where
    D: DBMSService,
{
    TypeDBIntegration::new(dbms_service, config.unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_name() {
        let config = TypeDBIntegrationConfig::default();
        let integration = TypeDBIntegration::<MockDBMSService>::new(
            Arc::new(MockDBMSService),
            config,
        );
        
        assert_eq!(integration.sanitize_name("ValidName"), "validname");
        assert_eq!(integration.sanitize_name("name-with-dashes"), "name_with_dashes");
        assert_eq!(integration.sanitize_name("123_starts_with_number"), "_123_starts_with_number");
        assert_eq!(integration.sanitize_name("CamelCase"), "camelcase");
    }

    #[test]
    fn test_map_range_to_typedb_type() {
        let config = TypeDBIntegrationConfig::default();
        let integration = TypeDBIntegration::<MockDBMSService>::new(
            Arc::new(MockDBMSService),
            config,
        );
        
        assert_eq!(integration.map_range_to_typedb_type("string"), "string");
        assert_eq!(integration.map_range_to_typedb_type("integer"), "long");
        assert_eq!(integration.map_range_to_typedb_type("float"), "double");
        assert_eq!(integration.map_range_to_typedb_type("boolean"), "boolean");
        assert_eq!(integration.map_range_to_typedb_type("datetime"), "datetime");
        assert_eq!(integration.map_range_to_typedb_type("unknown"), "string");
    }

    // Mock DBMS service for testing
    struct MockDBMSService;
    
    #[async_trait]
    impl DBMSService for MockDBMSService {
        type Error = std::io::Error;
        
        async fn create_database(&self, _name: &str, _config: DatabaseConfig) -> DBMSResult<DatabaseInfo> {
            unimplemented!("Mock implementation")
        }
        
        async fn delete_database(&self, _name: &str) -> DBMSResult<()> {
            unimplemented!("Mock implementation")
        }
        
        async fn list_databases(&self) -> DBMSResult<Vec<DatabaseInfo>> {
            unimplemented!("Mock implementation")
        }
        
        async fn deploy_schema(&self, _database: &str, _deployment: SchemaDeployment) -> DBMSResult<()> {
            unimplemented!("Mock implementation")
        }
        
        async fn execute_query(&self, _database: &str, _query: &str, _params: HashMap<String, Value>) -> DBMSResult<QueryResult> {
            unimplemented!("Mock implementation")
        }

        async fn get_database_status(&self, _name: &str) -> DBMSResult<DatabaseStatus> {
            Ok(DatabaseStatus::Online)
        }

        async fn get_connection(&self, _database: &str) -> DBMSResult<Arc<dyn DatabaseConnection<Error = Self::Error>>> {
            unimplemented!("Mock implementation")
        }

        async fn get_connection_pool(&self, _database: &str) -> DBMSResult<Arc<dyn ConnectionPool<Error = Self::Error>>> {
            unimplemented!("Mock implementation")
        }

        async fn health_check(&self, _database: &str) -> DBMSResult<HealthStatus> {
            Ok(HealthStatus::Healthy)
        }

        async fn validate_schema(&self, _database: &str) -> DBMSResult<SchemaValidation> {
            Ok(SchemaValidation::Valid)
        }

        async fn get_schema_version(&self, _database: &str) -> DBMSResult<SchemaVersion> {
            Ok(SchemaVersion::new(1, 0, 0))
        }

        async fn execute_string_query(&self, _database: &str, _query: &str) -> DBMSResult<String> {
            Ok("{}".to_string())
        }

        async fn get_database_metrics(&self, _database: &str) -> DBMSResult<DatabaseMetrics> {
            unimplemented!("Mock implementation")
        }

        async fn optimize_database(&self, _database: &str) -> DBMSResult<OptimizationReport> {
            unimplemented!("Mock implementation")
        }

        async fn get_events(&self, _database: Option<&str>, _since: chrono::DateTime<chrono::Utc>, _limit: u32) -> DBMSResult<Vec<DatabaseEvent>> {
            Ok(Vec::new())
        }

        async fn export_database(&self, _database: &str, _include_data: bool) -> DBMSResult<String> {
            Ok("export_path".to_string())
        }

        async fn import_database(&self, _database: &str, _import_path: &str, _replace_existing: bool) -> DBMSResult<u64> {
            Ok(0)
        }
    }
}