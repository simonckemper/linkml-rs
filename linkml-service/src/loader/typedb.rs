//! TypeDB loader and dumper for LinkML
//!
//! This module provides functionality to load data from TypeDB
//! and dump LinkML instances back to TypeDB using TypeQL.

use super::traits::{DataLoader, DataDumper, LoaderError, LoaderResult, DumperError, DumperResult, DataInstance};
use linkml_core::prelude::*;
use async_trait::async_trait;
use serde_json::{Value, Map};
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

/// Options for TypeDB loading and dumping
#[derive(Debug, Clone)]
pub struct TypeDBOptions {
    /// TypeDB server address
    pub server_address: String,
    
    /// Database name
    pub database_name: String,
    
    /// TypeQL type to LinkML class mapping
    pub type_mapping: HashMap<String, String>,
    
    /// TypeQL attribute to LinkML slot mapping (per type)
    pub attribute_mapping: HashMap<String, HashMap<String, String>>,
    
    /// Batch size for loading/dumping
    pub batch_size: usize,
    
    /// Whether to infer types from TypeDB schema
    pub infer_types: bool,
    
    /// Whether to create types if they don't exist
    pub create_if_not_exists: bool,
    
    /// Whether to use transactions
    pub use_transactions: bool,
    
    /// Session type (data or schema)
    pub session_type: SessionType,
    
    /// Transaction type (read or write)
    pub transaction_type: TransactionType,
    
    /// Include inferred attributes
    pub include_inferred: bool,
}

/// TypeDB session type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SessionType {
    Data,
    Schema,
}

/// TypeDB transaction type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransactionType {
    Read,
    Write,
}

impl Default for TypeDBOptions {
    fn default() -> Self {
        Self {
            server_address: "localhost:1729".to_string(),
            database_name: String::new(),
            type_mapping: HashMap::new(),
            attribute_mapping: HashMap::new(),
            batch_size: 1000,
            infer_types: true,
            create_if_not_exists: false,
            use_transactions: true,
            session_type: SessionType::Data,
            transaction_type: TransactionType::Read,
            include_inferred: false,
        }
    }
}

/// TypeDB loader for LinkML data
pub struct TypeDBLoader {
    options: TypeDBOptions,
    client: Option<TypeDBClient>,
}

impl TypeDBLoader {
    /// Create a new TypeDB loader
    pub fn new(options: TypeDBOptions) -> Self {
        Self {
            options,
            client: None,
        }
    }
    
    /// Connect to TypeDB
    async fn connect(&mut self) -> LoaderResult<()> {
        if self.client.is_none() {
            let client = TypeDBClient::new(&self.options.server_address)
                .await
                .map_err(|e| LoaderError::Io(std::io::Error::new(
                    std::io::ErrorKind::ConnectionRefused,
                    format!("Failed to connect to TypeDB: {}", e)
                )))?;
            
            self.client = Some(client);
        }
        Ok(())
    }
    
    /// Get the TypeDB client
    fn get_client(&self) -> LoaderResult<&TypeDBClient> {
        self.client.as_ref().ok_or_else(|| {
            LoaderError::Io(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "TypeDB not connected"
            ))
        })
    }
    
    /// Get entity types from TypeDB
    async fn get_entity_types(&self) -> LoaderResult<Vec<String>> {
        let client = self.get_client()?;
        let session = client.session(&self.options.database_name, SessionType::Schema).await?;
        let tx = session.transaction(TransactionType::Read).await?;
        
        let query = "match $x sub entity; get $x;";
        let answer_stream = tx.query().match_query(query).await?;
        
        let mut types = Vec::new();
        while let Some(answer) = answer_stream.try_next().await? {
            if let Some(concept) = answer.get("x") {
                if let Some(type_name) = concept.as_entity_type()?.label() {
                    if type_name != "entity" { // Skip the root type
                        types.push(type_name.to_string());
                    }
                }
            }
        }
        
        tx.close().await?;
        Ok(types)
    }
    
    /// Get relation types from TypeDB
    async fn get_relation_types(&self) -> LoaderResult<Vec<String>> {
        let client = self.get_client()?;
        let session = client.session(&self.options.database_name, SessionType::Schema).await?;
        let tx = session.transaction(TransactionType::Read).await?;
        
        let query = "match $x sub relation; get $x;";
        let answer_stream = tx.query().match_query(query).await?;
        
        let mut types = Vec::new();
        while let Some(answer) = answer_stream.try_next().await? {
            if let Some(concept) = answer.get("x") {
                if let Some(type_name) = concept.as_relation_type()?.label() {
                    if type_name != "relation" { // Skip the root type
                        types.push(type_name.to_string());
                    }
                }
            }
        }
        
        tx.close().await?;
        Ok(types)
    }
    
    /// Get attributes for a type
    async fn get_type_attributes(&self, type_name: &str) -> LoaderResult<Vec<AttributeInfo>> {
        let client = self.get_client()?;
        let session = client.session(&self.options.database_name, SessionType::Schema).await?;
        let tx = session.transaction(TransactionType::Read).await?;
        
        let query = format!("match ${} sub {}; ${} owns $attr; get $attr;", type_name, type_name, type_name);
        let answer_stream = tx.query().match_query(&query).await?;
        
        let mut attributes = Vec::new();
        while let Some(answer) = answer_stream.try_next().await? {
            if let Some(concept) = answer.get("attr") {
                if let Some(attr_type) = concept.as_attribute_type()? {
                    let label = attr_type.label()?.to_string();
                    let value_type = attr_type.value_type()?;
                    
                    attributes.push(AttributeInfo {
                        name: label,
                        value_type: value_type_to_string(value_type),
                    });
                }
            }
        }
        
        tx.close().await?;
        Ok(attributes)
    }
    
    /// Load instances of a specific type
    async fn load_type_instances(&self, type_name: &str, attributes: &[AttributeInfo], schema: &SchemaDefinition) 
        -> LoaderResult<Vec<DataInstance>> {
        let client = self.get_client()?;
        let session = client.session(&self.options.database_name, SessionType::Data).await?;
        let tx = session.transaction(TransactionType::Read).await?;
        
        // Get the class name for this type
        let class_name = self.options.type_mapping.get(type_name)
            .cloned()
            .unwrap_or_else(|| to_pascal_case(type_name));
        
        // Build match query
        let mut query = format!("match $x isa {};", type_name);
        for attr in attributes {
            query.push_str(&format!(" $x has {} ${}_;", attr.name, attr.name));
        }
        query.push_str(" get $x");
        for attr in attributes {
            query.push_str(&format!(", ${}_;", attr.name));
        }
        query.push(';');
        
        debug!("Executing query: {}", query);
        
        let answer_stream = tx.query().match_query(&query).await?;
        let mut instances = Vec::new();
        
        while let Some(answer) = answer_stream.try_next().await? {
            let mut data = Map::new();
            
            // Get attribute values
            for attr in attributes {
                let var_name = format!("{}_", attr.name);
                if let Some(concept) = answer.get(&var_name) {
                    if let Some(attribute) = concept.as_attribute()? {
                        let value = attribute_to_json_value(attribute)?;
                        
                        // Apply attribute mapping if exists
                        let slot_name = if let Some(mapping) = self.options.attribute_mapping.get(type_name) {
                            mapping.get(&attr.name)
                                .cloned()
                                .unwrap_or_else(|| to_snake_case(&attr.name))
                        } else {
                            to_snake_case(&attr.name)
                        };
                        
                        data.insert(slot_name, value);
                    }
                }
            }
            
            // Get the entity/relation ID
            if let Some(concept) = answer.get("x") {
                if let Some(thing) = concept.as_thing()? {
                    let iid = thing.iid();
                    data.insert("_typedb_iid".to_string(), Value::String(format!("{:?}", iid)));
                }
            }
            
            instances.push(DataInstance {
                class_name: class_name.clone(),
                data,
            });
        }
        
        tx.close().await?;
        Ok(instances)
    }
    
    /// Convert TypeDB value type to LinkML range
    fn value_type_to_linkml_range(value_type: &str) -> String {
        match value_type {
            "long" => "integer",
            "double" => "float",
            "boolean" => "boolean",
            "string" => "string",
            "datetime" => "datetime",
            _ => "string",
        }.to_string()
    }
}

#[async_trait]
impl DataLoader for TypeDBLoader {
    async fn load(&mut self, schema: &SchemaDefinition) -> LoaderResult<Vec<DataInstance>> {
        // Connect to TypeDB
        self.connect().await?;
        
        // Get all entity types
        let entity_types = self.get_entity_types().await?;
        info!("Found {} entity types", entity_types.len());
        
        // Get all relation types
        let relation_types = self.get_relation_types().await?;
        info!("Found {} relation types", relation_types.len());
        
        let mut all_instances = Vec::new();
        
        // Load entities
        for type_name in entity_types {
            debug!("Loading instances of entity type: {}", type_name);
            
            // Get attributes for this type
            let attributes = self.get_type_attributes(&type_name).await?;
            
            // Load instances
            let instances = self.load_type_instances(&type_name, &attributes, schema).await?;
            info!("Loaded {} instances of type {}", instances.len(), type_name);
            
            all_instances.extend(instances);
        }
        
        // Load relations
        for type_name in relation_types {
            debug!("Loading instances of relation type: {}", type_name);
            
            // Get attributes for this type
            let attributes = self.get_type_attributes(&type_name).await?;
            
            // Load instances
            let instances = self.load_type_instances(&type_name, &attributes, schema).await?;
            info!("Loaded {} instances of relation {}", instances.len(), type_name);
            
            all_instances.extend(instances);
        }
        
        Ok(all_instances)
    }
}

/// TypeDB dumper for LinkML data
pub struct TypeDBDumper {
    options: TypeDBOptions,
    client: Option<TypeDBClient>,
}

impl TypeDBDumper {
    /// Create a new TypeDB dumper
    pub fn new(options: TypeDBOptions) -> Self {
        Self {
            options,
            client: None,
        }
    }
    
    /// Connect to TypeDB
    async fn connect(&mut self) -> DumperResult<()> {
        if self.client.is_none() {
            let client = TypeDBClient::new(&self.options.server_address)
                .await
                .map_err(|e| DumperError::Io(std::io::Error::new(
                    std::io::ErrorKind::ConnectionRefused,
                    format!("Failed to connect to TypeDB: {}", e)
                )))?;
            
            self.client = Some(client);
        }
        Ok(())
    }
    
    /// Get the TypeDB client
    fn get_client(&self) -> DumperResult<&TypeDBClient> {
        self.client.as_ref().ok_or_else(|| {
            DumperError::Io(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "TypeDB not connected"
            ))
        })
    }
    
    /// Create type if needed
    async fn create_type_if_needed(&self, class_name: &str, class_def: &ClassDefinition, 
                                   schema: &SchemaDefinition) -> DumperResult<()> {
        if !self.options.create_if_not_exists {
            return Ok(());
        }
        
        let client = self.get_client()?;
        let session = client.session(&self.options.database_name, SessionType::Schema).await?;
        let tx = session.transaction(TransactionType::Write).await?;
        
        // Get TypeDB type name
        let type_name = self.options.type_mapping.iter()
            .find(|(_, cn)| cn == &class_name)
            .map(|(tn, _)| tn.clone())
            .unwrap_or_else(|| to_snake_case(class_name));
        
        // Check if any slots are object-valued (indicating a relation)
        let is_relation = class_def.slots.iter().any(|slot_name| {
            schema.slots.get(slot_name)
                .and_then(|slot| slot.range.as_ref())
                .map(|range| schema.classes.contains_key(range))
                .unwrap_or(false)
        });
        
        // Create type definition
        let type_kind = if is_relation { "relation" } else { "entity" };
        let mut query = format!("define {} sub {};", type_name, type_kind);
        
        // Add attributes
        for slot_name in &class_def.slots {
            if let Some(slot_def) = schema.slots.get(slot_name) {
                if let Some(range) = &slot_def.range {
                    // Skip object-valued slots for entities
                    if !is_relation && schema.classes.contains_key(range) {
                        continue;
                    }
                    
                    let attr_name = to_snake_case(slot_name);
                    let value_type = linkml_range_to_typedb_value_type(range);
                    
                    // Define attribute if needed
                    query.push_str(&format!(" {} sub attribute, value {};", attr_name, value_type));
                    
                    // Type owns attribute
                    query.push_str(&format!(" {} owns {};", type_name, attr_name));
                }
            }
        }
        
        tx.query().define_query(&query).await?;
        tx.commit().await?;
        
        Ok(())
    }
    
    /// Insert instances for a class
    async fn insert_instances(&self, class_name: &str, instances: &[DataInstance], 
                             schema: &SchemaDefinition) -> DumperResult<()> {
        let client = self.get_client()?;
        
        // Get type name
        let type_name = self.options.type_mapping.iter()
            .find(|(_, cn)| cn == &class_name)
            .map(|(tn, _)| tn.clone())
            .unwrap_or_else(|| to_snake_case(class_name));
        
        // Get class definition
        let class_def = schema.classes.get(class_name)
            .ok_or_else(|| DumperError::ValidationFailed(
                format!("Class {} not found in schema", class_name)
            ))?;
        
        // Check if this is a relation
        let is_relation = class_def.slots.iter().any(|slot_name| {
            schema.slots.get(slot_name)
                .and_then(|slot| slot.range.as_ref())
                .map(|range| schema.classes.contains_key(range))
                .unwrap_or(false)
        });
        
        // Process in batches
        for batch in instances.chunks(self.options.batch_size) {
            let session = client.session(&self.options.database_name, SessionType::Data).await?;
            let tx = session.transaction(TransactionType::Write).await?;
            
            for instance in batch {
                // Build insert query
                let mut query = String::new();
                
                if is_relation {
                    // Handle relation insertion
                    query.push_str(&format!("match "));
                    
                    // Match role players
                    let mut role_players = Vec::new();
                    for (slot_name, value) in &instance.data {
                        if let Some(slot_def) = schema.slots.get(slot_name) {
                            if let Some(range) = &slot_def.range {
                                if schema.classes.contains_key(range) {
                                    // This is a role player
                                    if let Value::Object(obj) = value {
                                        if let Some(Value::String(id)) = obj.get("id") {
                                            query.push_str(&format!("${} isa {} has id \"{}\"; ", 
                                                slot_name, to_snake_case(range), id));
                                            role_players.push(slot_name.clone());
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    query.push_str(&format!("insert $rel ({}) isa {};", 
                        role_players.iter()
                            .map(|rp| format!("{}: ${}", rp, rp))
                            .collect::<Vec<_>>()
                            .join(", "),
                        type_name
                    ));
                } else {
                    // Handle entity insertion
                    query.push_str(&format!("insert $x isa {}", type_name));
                }
                
                // Add attributes
                for (slot_name, value) in &instance.data {
                    // Skip TypeDB internal fields
                    if slot_name.starts_with('_') {
                        continue;
                    }
                    
                    // Skip object-valued slots
                    if let Some(slot_def) = schema.slots.get(slot_name) {
                        if let Some(range) = &slot_def.range {
                            if schema.classes.contains_key(range) {
                                continue;
                            }
                        }
                    }
                    
                    let attr_name = to_snake_case(slot_name);
                    let typeql_value = json_value_to_typeql(value)?;
                    
                    if is_relation {
                        query.push_str(&format!(", has {} {}", attr_name, typeql_value));
                    } else {
                        query.push_str(&format!(", has {} {}", attr_name, typeql_value));
                    }
                }
                
                query.push(';');
                
                tx.query().insert_query(&query).await?;
            }
            
            tx.commit().await?;
        }
        
        Ok(())
    }
}

#[async_trait]
impl DataDumper for TypeDBDumper {
    async fn dump(&mut self, instances: &[DataInstance], schema: &SchemaDefinition) -> DumperResult<Vec<u8>> {
        // Connect to TypeDB
        self.connect().await?;
        
        // Group instances by class
        let mut instances_by_class: HashMap<String, Vec<&DataInstance>> = HashMap::new();
        for instance in instances {
            instances_by_class.entry(instance.class_name.clone())
                .or_default()
                .push(instance);
        }
        
        // Create types if needed and insert data
        for (class_name, class_instances) in instances_by_class {
            if let Some(class_def) = schema.classes.get(&class_name) {
                // Create type if needed
                self.create_type_if_needed(&class_name, class_def, schema).await?;
                
                // Convert references to owned instances
                let owned_instances: Vec<DataInstance> = class_instances.into_iter()
                    .cloned()
                    .collect();
                
                // Insert instances
                self.insert_instances(&class_name, &owned_instances, schema).await?;
                
                info!("Dumped {} instances of class {}", owned_instances.len(), class_name);
            }
        }
        
        // Return summary as bytes
        let summary = format!("Successfully dumped {} instances to TypeDB", instances.len());
        Ok(summary.into_bytes())
    }
}

/// Attribute information
#[derive(Debug, Clone)]
struct AttributeInfo {
    name: String,
    value_type: String,
}

/// Convert string to pascal case
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

/// Convert string to snake case
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut prev_upper = false;
    
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 && !prev_upper {
            result.push('_');
        }
        // to_lowercase() for ASCII characters always produces at least one char
        result.push(ch.to_lowercase().next().expect("to_lowercase() should produce at least one char"));
        prev_upper = ch.is_uppercase();
    }
    
    result
}

/// Convert TypeDB value type to string
fn value_type_to_string(value_type: ValueType) -> String {
    match value_type {
        ValueType::Long => "long",
        ValueType::Double => "double",
        ValueType::Boolean => "boolean",
        ValueType::String => "string",
        ValueType::DateTime => "datetime",
    }.to_string()
}

/// Convert LinkML range to TypeDB value type
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

/// Convert TypeDB attribute to JSON value
fn attribute_to_json_value(attribute: &Attribute) -> LoaderResult<Value> {
    let value = match attribute.value() {
        AttributeValue::Long(v) => Value::Number(serde_json::Number::from(v)),
        AttributeValue::Double(v) => {
            serde_json::Number::from_f64(v)
                .map(Value::Number)
                .unwrap_or(Value::Null)
        }
        AttributeValue::Boolean(v) => Value::Bool(v),
        AttributeValue::String(v) => Value::String(v),
        AttributeValue::DateTime(v) => Value::String(v.to_rfc3339()),
    };
    Ok(value)
}

/// Convert JSON value to TypeQL literal
fn json_value_to_typeql(value: &Value) -> DumperResult<String> {
    match value {
        Value::String(s) => Ok(format!("\"{}\"", s.replace('\"', "\\\""))),
        Value::Number(n) => Ok(n.to_string()),
        Value::Bool(b) => Ok(b.to_string()),
        Value::Null => Err(DumperError::ValidationFailed("Cannot insert null values into TypeDB".to_string())),
        _ => Err(DumperError::ValidationFailed(format!("Unsupported value type: {:?}", value))),
    }
}

// TypeDB client mock for compilation
// In real implementation, this would use typedb-driver
struct TypeDBClient;
struct Session;
struct Transaction;
struct AnswerStream;
struct Answer;
struct Concept;
struct EntityType;
struct RelationType;
struct AttributeType;
struct Thing;
struct Attribute;
struct Query;

#[derive(Debug)]
enum ValueType {
    Long,
    Double,
    Boolean,
    String,
    DateTime,
}

#[derive(Debug)]
enum AttributeValue {
    Long(i64),
    Double(f64),
    Boolean(bool),
    String(String),
    DateTime(chrono::DateTime<chrono::Utc>),
}

// Mock implementations for compilation
impl TypeDBClient {
    async fn new(_address: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self)
    }
    
    async fn session(&self, _database: &str, _session_type: SessionType) -> Result<Session, Box<dyn std::error::Error>> {
        Ok(Session)
    }
}

impl Session {
    async fn transaction(&self, _tx_type: TransactionType) -> Result<Transaction, Box<dyn std::error::Error>> {
        Ok(Transaction)
    }
}

impl Transaction {
    fn query(&self) -> Query {
        Query
    }
    
    async fn close(&self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
    
    async fn commit(&self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

impl Query {
    async fn match_query(&self, _query: &str) -> Result<AnswerStream, Box<dyn std::error::Error>> {
        Ok(AnswerStream)
    }
    
    async fn define_query(&self, _query: &str) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
    
    async fn insert_query(&self, _query: &str) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

// Stream trait mock
use futures::stream::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

impl Stream for AnswerStream {
    type Item = Result<Answer, Box<dyn std::error::Error>>;
    
    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Ready(None)
    }
}

// TryStreamExt for async iteration
use futures::TryStreamExt;

impl Answer {
    fn get(&self, _var: &str) -> Option<&Concept> {
        None
    }
}

impl Concept {
    fn as_entity_type(&self) -> Result<&EntityType, Box<dyn std::error::Error>> {
        Ok(&EntityType)
    }
    
    fn as_relation_type(&self) -> Result<&RelationType, Box<dyn std::error::Error>> {
        Ok(&RelationType)
    }
    
    fn as_attribute_type(&self) -> Result<&AttributeType, Box<dyn std::error::Error>> {
        Ok(&AttributeType)
    }
    
    fn as_thing(&self) -> Result<&Thing, Box<dyn std::error::Error>> {
        Ok(&Thing)
    }
    
    fn as_attribute(&self) -> Result<&Attribute, Box<dyn std::error::Error>> {
        Ok(&Attribute)
    }
}

impl EntityType {
    fn label(&self) -> Result<&str, Box<dyn std::error::Error>> {
        Ok("entity")
    }
}

impl RelationType {
    fn label(&self) -> Result<&str, Box<dyn std::error::Error>> {
        Ok("relation")
    }
}

impl AttributeType {
    fn label(&self) -> Result<&str, Box<dyn std::error::Error>> {
        Ok("attribute")
    }
    
    fn value_type(&self) -> Result<ValueType, Box<dyn std::error::Error>> {
        Ok(ValueType::String)
    }
}

impl Thing {
    fn iid(&self) -> [u8; 16] {
        [0; 16]
    }
}

impl Attribute {
    fn value(&self) -> AttributeValue {
        AttributeValue::String("".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_typedb_options_default() {
        let options = TypeDBOptions::default();
        
        assert_eq!(options.server_address, "localhost:1729");
        assert_eq!(options.batch_size, 1000);
        assert!(options.infer_types);
        assert!(!options.create_if_not_exists);
        assert!(options.use_transactions);
        assert_eq!(options.session_type, SessionType::Data);
        assert_eq!(options.transaction_type, TransactionType::Read);
        assert!(!options.include_inferred);
    }
    
    #[test]
    fn test_pascal_case_conversion() {
        assert_eq!(to_pascal_case("user_account"), "UserAccount");
        assert_eq!(to_pascal_case("person"), "Person");
        assert_eq!(to_pascal_case("has_address"), "HasAddress");
    }
    
    #[test]
    fn test_snake_case_conversion() {
        assert_eq!(to_snake_case("UserAccount"), "user_account");
        assert_eq!(to_snake_case("Person"), "person");
        assert_eq!(to_snake_case("hasAddress"), "has_address");
    }
    
    #[test]
    fn test_type_mapping() {
        let options = TypeDBOptions {
            server_address: "localhost:1729".to_string(),
            database_name: "test".to_string(),
            type_mapping: [
                ("person".to_string(), "Person".to_string()),
                ("employment".to_string(), "Employment".to_string()),
            ].into_iter().collect(),
            ..Default::default()
        };
        
        assert_eq!(options.type_mapping.get("person"), Some(&"Person".to_string()));
        assert_eq!(options.type_mapping.get("employment"), Some(&"Employment".to_string()));
    }
    
    #[test]
    fn test_value_type_conversion() {
        assert_eq!(linkml_range_to_typedb_value_type("integer"), "long");
        assert_eq!(linkml_range_to_typedb_value_type("float"), "double");
        assert_eq!(linkml_range_to_typedb_value_type("boolean"), "boolean");
        assert_eq!(linkml_range_to_typedb_value_type("string"), "string");
        assert_eq!(linkml_range_to_typedb_value_type("datetime"), "datetime");
        assert_eq!(linkml_range_to_typedb_value_type("unknown"), "string");
    }
}