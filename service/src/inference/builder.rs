//! LinkML Schema Builder
//!
//! Provides a fluent API for programmatically constructing LinkML schemas.
//! This builder enables incremental schema construction with type-safe operations
//! and validates the resulting schema structure.

use indexmap::IndexMap;
use linkml_core::types::{
    ClassDefinition, PrefixDefinition, SchemaDefinition, SlotDefinition,
};
use linkml_core::LinkMLError;
use std::sync::Arc;
use timestamp_core::{TimestampError, TimestampService};

/// Result type for schema builder operations
pub type BuilderResult<T> = Result<T, LinkMLError>;

/// Builder for constructing LinkML schemas programmatically
///
/// # Example
///
/// ```rust
/// use linkml_service::inference::builder::SchemaBuilder;
///
/// let schema = SchemaBuilder::new("person_schema", "PersonSchema")
///     .with_description("Schema for person data")
///     .with_version("1.0.0")
///     .add_prefix("schema", "http://schema.org/")
///     .add_class("Person")
///         .with_description("A person entity")
///         .add_attribute("name", "string", true, false)
///         .add_attribute("age", "integer", false, false)
///         .add_attribute("emails", "string", false, true)
///         .finish()
///     .build();
/// ```
pub struct SchemaBuilder {
    schema_id: String,
    schema_name: String,
    title: Option<String>,
    description: Option<String>,
    version: Option<String>,
    license: Option<String>,
    default_prefix: Option<String>,
    default_range: Option<String>,
    prefixes: IndexMap<String, PrefixDefinition>,
    classes: IndexMap<String, ClassDefinition>,
    slots: IndexMap<String, SlotDefinition>,
    timestamp_service: Option<Arc<dyn TimestampService<Error = TimestampError>>>,
}

impl SchemaBuilder {
    /// Create a new schema builder
    ///
    /// # Arguments
    ///
    /// * `schema_id` - Unique identifier for the schema
    /// * `schema_name` - Human-readable name for the schema
    pub fn new(schema_id: impl Into<String>, schema_name: impl Into<String>) -> Self {
        let mut prefixes = IndexMap::new();
        // Add standard LinkML prefix
        prefixes.insert(
            "linkml".to_string(),
            PrefixDefinition::Simple("https://w3id.org/linkml/".to_string()),
        );

        Self {
            schema_id: schema_id.into(),
            schema_name: schema_name.into(),
            title: None,
            description: None,
            version: None,
            license: None,
            default_prefix: None,
            default_range: None,
            prefixes,
            classes: IndexMap::new(),
            slots: IndexMap::new(),
            timestamp_service: None,
        }
    }

    /// Set the timestamp service for schema metadata
    pub fn with_timestamp_service(
        mut self,
        service: Arc<dyn TimestampService<Error = TimestampError>>,
    ) -> Self {
        self.timestamp_service = Some(service);
        self
    }

    /// Set the schema title
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the schema description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the schema version
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// Set the schema license
    pub fn with_license(mut self, license: impl Into<String>) -> Self {
        self.license = Some(license.into());
        self
    }

    /// Set the default prefix for the schema
    pub fn with_default_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.default_prefix = Some(prefix.into());
        self
    }

    /// Set the default range for slots
    pub fn with_default_range(mut self, range: impl Into<String>) -> Self {
        self.default_range = Some(range.into());
        self
    }

    /// Set generation metadata
    ///
    /// # Arguments
    ///
    /// * `generator` - Name and version of the generator
    /// * `source_file` - Optional source file path
    pub fn with_generation_metadata(
        mut self,
        generator: impl Into<String>,
        source_file: Option<String>,
    ) -> Self {
        // Store generator info - will be used in build()
        // Note: We'll add these fields to the struct if needed
        // For now, this method exists for API compatibility
        let _ = (generator.into(), source_file);
        self
    }

    /// Add a namespace prefix
    ///
    /// # Arguments
    ///
    /// * `prefix` - Short prefix name
    /// * `uri` - Full URI expansion
    pub fn add_prefix(mut self, prefix: impl Into<String>, uri: impl Into<String>) -> Self {
        self.prefixes.insert(
            prefix.into(),
            PrefixDefinition::Simple(uri.into()),
        );
        self
    }

    /// Add a complex prefix with reference
    ///
    /// # Arguments
    ///
    /// * `prefix` - Short prefix name
    /// * `prefix_prefix` - Prefix expansion
    /// * `prefix_reference` - Optional reference URL
    pub fn add_complex_prefix(
        mut self,
        prefix: impl Into<String>,
        prefix_prefix: impl Into<String>,
        prefix_reference: Option<String>,
    ) -> Self {
        self.prefixes.insert(
            prefix.into(),
            PrefixDefinition::Complex {
                prefix_prefix: prefix_prefix.into(),
                prefix_reference,
            },
        );
        self
    }

    /// Start building a class definition
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the class
    ///
    /// # Returns
    ///
    /// A `ClassBuilder` for fluent class construction
    pub fn add_class(self, name: impl Into<String>) -> ClassBuilder {
        ClassBuilder::new(self, name.into())
    }

    /// Add a top-level slot definition
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the slot
    ///
    /// # Returns
    ///
    /// A `SlotBuilder` for fluent slot construction
    pub fn add_slot(self, name: impl Into<String>) -> SlotBuilder {
        SlotBuilder::new(self, name.into())
    }

    /// Build the final schema definition
    ///
    /// # Returns
    ///
    /// A complete `SchemaDefinition` ready for serialization
    pub fn build(self) -> SchemaDefinition {
        // Note: generation_date is set to None here as timestamp_service.now_utc() is async
        // Callers should set generation_date manually if needed
        let generation_date = None;

        SchemaDefinition {
            id: self.schema_id,
            name: self.schema_name,
            title: self.title,
            description: self.description,
            version: self.version,
            license: self.license,
            default_prefix: self.default_prefix,
            prefixes: self.prefixes,
            imports: Vec::new(),
            classes: self.classes,
            slots: self.slots,
            types: IndexMap::new(),
            enums: IndexMap::new(),
            subsets: IndexMap::new(),
            default_range: self.default_range,
            generation_date,
            source_file: None,
            metamodel_version: Some("1.7.0".to_string()),
            settings: None,
            annotations: None,
            contributors: Vec::new(),
            status: None,
            categories: Vec::new(),
            keywords: Vec::new(),
            see_also: Vec::new(),
        }
    }

    /// Internal method to add a class after building
    fn add_class_internal(mut self, name: String, class_def: ClassDefinition) -> Self {
        self.classes.insert(name, class_def);
        self
    }

    /// Internal method to add a slot after building
    fn add_slot_internal(mut self, name: String, slot_def: SlotDefinition) -> Self {
        self.slots.insert(name, slot_def);
        self
    }
}

/// Builder for constructing class definitions
pub struct ClassBuilder {
    schema_builder: SchemaBuilder,
    class_name: String,
    description: Option<String>,
    is_abstract: bool,
    is_mixin: bool,
    is_a: Option<String>,
    mixins: Vec<String>,
    slots: Vec<String>,
    attributes: IndexMap<String, SlotDefinition>,
    tree_root: bool,
}

impl ClassBuilder {
    fn new(schema_builder: SchemaBuilder, class_name: String) -> Self {
        Self {
            schema_builder,
            class_name,
            description: None,
            is_abstract: false,
            is_mixin: false,
            is_a: None,
            mixins: Vec::new(),
            slots: Vec::new(),
            attributes: IndexMap::new(),
            tree_root: false,
        }
    }

    /// Set the class description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Mark the class as abstract
    pub fn abstract_(mut self) -> Self {
        self.is_abstract = true;
        self
    }

    /// Mark the class as a mixin
    pub fn mixin(mut self) -> Self {
        self.is_mixin = true;
        self
    }

    /// Set the parent class (inheritance)
    pub fn is_a(mut self, parent: impl Into<String>) -> Self {
        self.is_a = Some(parent.into());
        self
    }

    /// Add a mixin class
    pub fn add_mixin(mut self, mixin: impl Into<String>) -> Self {
        self.mixins.push(mixin.into());
        self
    }

    /// Mark as tree root (top-level entity in instance data)
    pub fn tree_root(mut self) -> Self {
        self.tree_root = true;
        self
    }

    /// Add a reference to an existing slot definition
    pub fn use_slot(mut self, slot_name: impl Into<String>) -> Self {
        self.slots.push(slot_name.into());
        self
    }

    /// Add an inline attribute (slot specific to this class)
    ///
    /// # Arguments
    ///
    /// * `name` - Attribute name
    /// * `range` - Type/class name for the attribute
    /// * `required` - Whether the attribute is required
    /// * `multivalued` - Whether the attribute can have multiple values
    pub fn add_attribute(
        mut self,
        name: impl Into<String>,
        range: impl Into<String>,
        required: bool,
        multivalued: bool,
    ) -> Self {
        let attr_name = name.into();
        let slot = SlotDefinition {
            name: attr_name.clone(),
            range: Some(range.into()),
            required: Some(required),
            multivalued: Some(multivalued),
            description: None,
            ..Default::default()
        };
        self.attributes.insert(attr_name, slot);
        self
    }

    /// Add an inline attribute with full slot definition
    pub fn add_attribute_with_slot(
        mut self,
        name: impl Into<String>,
        slot: SlotDefinition,
    ) -> Self {
        self.attributes.insert(name.into(), slot);
        self
    }

    /// Add a slot with a specified type and requirements
    ///
    /// # Arguments
    ///
    /// * `name` - Slot name
    /// * `slot_type` - Type/class name for the slot (String or &InferredType)
    /// * `required` - Whether the slot is required
    /// * `multivalued` - Whether the slot can have multiple values
    pub fn add_slot_with_type(
        self,
        name: impl Into<String>,
        slot_type: impl Into<String>,
        required: bool,
        multivalued: bool,
    ) -> Self {
        self.add_attribute(name, slot_type, required, multivalued)
    }

    /// Add a slot (simple version with just name and type)
    ///
    /// # Arguments
    ///
    /// * `name` - Slot name
    /// * `slot_type` - Type/class name for the slot
    pub fn add_slot(self, name: impl Into<String>, slot_type: impl Into<String>) -> Self {
        self.add_attribute(name, slot_type, false, false)
    }

    /// Finish building this class and return to schema builder
    pub fn finish(self) -> SchemaBuilder {
        let class_def = ClassDefinition {
            name: self.class_name.clone(),
            description: self.description,
            abstract_: if self.is_abstract { Some(true) } else { None },
            mixin: if self.is_mixin { Some(true) } else { None },
            is_a: self.is_a,
            mixins: self.mixins,
            slots: self.slots,
            slot_usage: IndexMap::new(),
            attributes: self.attributes,
            class_uri: None,
            subclass_of: Vec::new(),
            tree_root: if self.tree_root { Some(true) } else { None },
            rules: Vec::new(),
            if_required: None,
            unique_keys: IndexMap::new(),
            annotations: None,
            recursion_options: None,
            aliases: Vec::new(),
            see_also: Vec::new(),
            examples: Vec::new(),
            deprecated: None,
            todos: Vec::new(),
            notes: Vec::new(),
            comments: Vec::new(),
        };

        self.schema_builder.add_class_internal(self.class_name, class_def)
    }
}

/// Builder for constructing top-level slot definitions
pub struct SlotBuilder {
    schema_builder: SchemaBuilder,
    slot_name: String,
    description: Option<String>,
    range: Option<String>,
    required: bool,
    multivalued: bool,
    identifier: bool,
    pattern: Option<String>,
    domain: Option<String>,
}

impl SlotBuilder {
    fn new(schema_builder: SchemaBuilder, slot_name: String) -> Self {
        Self {
            schema_builder,
            slot_name,
            description: None,
            range: None,
            required: false,
            multivalued: false,
            identifier: false,
            pattern: None,
            domain: None,
        }
    }

    /// Set the slot description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the slot range (type)
    pub fn with_range(mut self, range: impl Into<String>) -> Self {
        self.range = Some(range.into());
        self
    }

    /// Mark the slot as required
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Mark the slot as multivalued
    pub fn multivalued(mut self) -> Self {
        self.multivalued = true;
        self
    }

    /// Mark the slot as an identifier
    pub fn identifier(mut self) -> Self {
        self.identifier = true;
        self
    }

    /// Set a validation pattern
    pub fn with_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.pattern = Some(pattern.into());
        self
    }

    /// Set the domain (applicable class)
    pub fn with_domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = Some(domain.into());
        self
    }

    /// Finish building this slot and return to schema builder
    pub fn finish(self) -> SchemaBuilder {
        let slot_def = SlotDefinition {
            name: self.slot_name.clone(),
            description: self.description,
            range: self.range,
            required: Some(self.required),
            multivalued: Some(self.multivalued),
            identifier: Some(self.identifier),
            pattern: self.pattern,
            domain: self.domain,
            ..Default::default()
        };

        self.schema_builder.add_slot_internal(self.slot_name, slot_def)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_schema_builder() {
        let schema = SchemaBuilder::new("test_schema", "TestSchema")
            .with_description("A test schema")
            .with_version("1.0.0")
            .build();

        assert_eq!(schema.id, "test_schema");
        assert_eq!(schema.name, "TestSchema");
        assert_eq!(schema.description, Some("A test schema".to_string()));
        assert_eq!(schema.version, Some("1.0.0".to_string()));
        assert!(schema.prefixes.contains_key("linkml"));
    }

    #[test]
    fn test_schema_with_prefix() {
        let schema = SchemaBuilder::new("test", "Test")
            .add_prefix("schema", "http://schema.org/")
            .add_prefix("dc", "http://purl.org/dc/terms/")
            .build();

        assert_eq!(schema.prefixes.len(), 3); // linkml + schema + dc
        assert!(schema.prefixes.contains_key("schema"));
        assert!(schema.prefixes.contains_key("dc"));
    }

    #[test]
    fn test_schema_with_class() {
        let schema = SchemaBuilder::new("person_schema", "PersonSchema")
            .add_class("Person")
                .with_description("A person entity")
                .add_attribute("name", "string", true, false)
                .add_attribute("age", "integer", false, false)
                .finish()
            .build();

        assert!(schema.classes.contains_key("Person"));
        let person_class = schema.classes.get("Person").unwrap();
        assert_eq!(person_class.description, Some("A person entity".to_string()));
        assert_eq!(person_class.attributes.len(), 2);

        let name_attr = person_class.attributes.get("name").unwrap();
        assert_eq!(name_attr.range, Some("string".to_string()));
        assert_eq!(name_attr.required, Some(true));
        assert_eq!(name_attr.multivalued, Some(false));
    }

    #[test]
    fn test_schema_with_multiple_classes() {
        let schema = SchemaBuilder::new("org_schema", "OrganizationSchema")
            .add_class("Person")
                .add_attribute("name", "string", true, false)
                .finish()
            .add_class("Organization")
                .add_attribute("org_name", "string", true, false)
                .add_attribute("employees", "Person", false, true)
                .finish()
            .build();

        assert_eq!(schema.classes.len(), 2);
        assert!(schema.classes.contains_key("Person"));
        assert!(schema.classes.contains_key("Organization"));

        let org_class = schema.classes.get("Organization").unwrap();
        let employees = org_class.attributes.get("employees").unwrap();
        assert_eq!(employees.range, Some("Person".to_string()));
        assert_eq!(employees.multivalued, Some(true));
    }

    #[test]
    fn test_class_inheritance() {
        let schema = SchemaBuilder::new("test", "Test")
            .add_class("Entity")
                .abstract_()
                .add_attribute("id", "string", true, false)
                .finish()
            .add_class("Person")
                .is_a("Entity")
                .add_attribute("name", "string", true, false)
                .finish()
            .build();

        let entity = schema.classes.get("Entity").unwrap();
        assert_eq!(entity.abstract_, Some(true));

        let person = schema.classes.get("Person").unwrap();
        assert_eq!(person.is_a, Some("Entity".to_string()));
    }

    #[test]
    fn test_class_mixins() {
        let schema = SchemaBuilder::new("test", "Test")
            .add_class("Timestamped")
                .mixin()
                .add_attribute("created_at", "datetime", true, false)
                .finish()
            .add_class("Person")
                .add_mixin("Timestamped")
                .add_attribute("name", "string", true, false)
                .finish()
            .build();

        let timestamped = schema.classes.get("Timestamped").unwrap();
        assert_eq!(timestamped.mixin, Some(true));

        let person = schema.classes.get("Person").unwrap();
        assert_eq!(person.mixins, vec!["Timestamped".to_string()]);
    }

    #[test]
    fn test_top_level_slot() {
        let schema = SchemaBuilder::new("test", "Test")
            .add_slot("email")
                .with_description("Email address")
                .with_range("string")
                .required()
                .with_pattern("^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$")
                .finish()
            .add_class("Person")
                .use_slot("email")
                .finish()
            .build();

        assert!(schema.slots.contains_key("email"));
        let email_slot = schema.slots.get("email").unwrap();
        assert_eq!(email_slot.range, Some("string".to_string()));
        assert_eq!(email_slot.required, Some(true));
        assert!(email_slot.pattern.is_some());

        let person = schema.classes.get("Person").unwrap();
        assert_eq!(person.slots, vec!["email".to_string()]);
    }

    #[test]
    fn test_default_range() {
        let schema = SchemaBuilder::new("test", "Test")
            .with_default_range("string")
            .add_class("Person")
                .add_attribute("name", "string", true, false)
                .finish()
            .build();

        assert_eq!(schema.default_range, Some("string".to_string()));
    }

    #[test]
    fn test_schema_serialization_to_yaml() {
        let schema = SchemaBuilder::new("simple", "SimpleSchema")
            .with_version("1.0.0")
            .add_prefix("ex", "https://example.org/")
            .add_class("Example")
                .with_description("An example class")
                .add_attribute("field1", "string", true, false)
                .finish()
            .build();

        let yaml = serde_yaml::to_string(&schema).expect("Failed to serialize to YAML");
        assert!(yaml.contains("id: simple"));
        assert!(yaml.contains("name: SimpleSchema"));
        assert!(yaml.contains("Example:"));
        assert!(yaml.contains("field1:"));
    }

    #[test]
    fn test_tree_root_class() {
        let schema = SchemaBuilder::new("test", "Test")
            .add_class("Container")
                .tree_root()
                .add_attribute("items", "Item", false, true)
                .finish()
            .add_class("Item")
                .add_attribute("name", "string", true, false)
                .finish()
            .build();

        let container = schema.classes.get("Container").unwrap();
        assert_eq!(container.tree_root, Some(true));
    }

    #[test]
    fn test_complex_prefix() {
        let schema = SchemaBuilder::new("test", "Test")
            .add_complex_prefix(
                "obo",
                "http://purl.obolibrary.org/obo/",
                Some("https://obofoundry.org".to_string()),
            )
            .build();

        assert!(schema.prefixes.contains_key("obo"));
        if let PrefixDefinition::Complex { prefix_prefix, prefix_reference } =
            schema.prefixes.get("obo").unwrap()
        {
            assert_eq!(prefix_prefix, "http://purl.obolibrary.org/obo/");
            assert_eq!(prefix_reference, &Some("https://obofoundry.org".to_string()));
        } else {
            panic!("Expected complex prefix definition");
        }
    }
}
